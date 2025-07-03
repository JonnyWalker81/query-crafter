use std::net::TcpListener;
use std::process::Stdio;

use aws_config::BehaviorVersion;
use aws_sdk_ec2::types::{Filter, Instance};
use color_eyre::eyre::{anyhow, Result};
use tokio::process::{Child, Command};
use tokio::time::{sleep, timeout, Duration};

#[derive(Debug)]
pub struct TunnelConfig {
    pub environment: String,
    pub aws_profile: Option<String>,
    pub bastion_user: String,
    pub ssh_key: Option<String>,
    pub database_name: String,
    pub use_session_manager: bool,
}

#[derive(Debug)]
pub struct TunnelManager {
    config: TunnelConfig,
    ssh_process: Option<Child>,
    local_port: Option<u16>,
    remote_host: Option<String>,
    remote_port: u16,
}

impl TunnelManager {
    pub fn new(config: TunnelConfig) -> Self {
        Self {
            config,
            ssh_process: None,
            local_port: None,
            remote_host: None,
            remote_port: 5432, // Default PostgreSQL port
        }
    }

    /// Find an available local port
    fn find_available_port() -> Result<u16> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        drop(listener);
        Ok(port)
    }

    /// Find AWS CLI executable path
    fn find_aws_cli_path() -> Result<String> {
        // Check environment variable first
        if let Ok(aws_path) = std::env::var("AWS_CLI_PATH") {
            if std::path::Path::new(&aws_path).exists() {
                return Ok(aws_path);
            }
        }
        
        // Try common locations first
        let common_paths = vec![
            "/usr/local/bin/aws",
            "/usr/bin/aws",
            "/opt/homebrew/bin/aws",
            "/home/linuxbrew/.linuxbrew/bin/aws",
            "/nix/var/nix/profiles/default/bin/aws",
            "/run/current-system/sw/bin/aws",
        ];
        
        // Also check user's nix profile
        if let Ok(home) = std::env::var("HOME") {
            let nix_profile_paths = vec![
                format!("{}/.nix-profile/bin/aws", home),
                format!("{}/nix-profile/bin/aws", home),
            ];
            for path in nix_profile_paths {
                if std::path::Path::new(&path).exists() {
                    return Ok(path);
                }
            }
        }
        
        for path in common_paths {
            if std::path::Path::new(path).exists() {
                return Ok(path.to_string());
            }
        }
        
        // Try to find it using which command
        let output = std::process::Command::new("which")
            .arg("aws")
            .output();
            
        if let Ok(output) = output {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Ok(path);
                }
            }
        }
        
        // Try to find in PATH
        if let Ok(path_env) = std::env::var("PATH") {
            for path_dir in path_env.split(':') {
                let aws_path = format!("{}/aws", path_dir);
                if std::path::Path::new(&aws_path).exists() {
                    return Ok(aws_path);
                }
            }
        }
        
        // Last resort - try to find in user's home directory
        if let Ok(home) = std::env::var("HOME") {
            let user_paths = vec![
                format!("{}/.local/bin/aws", home),
                format!("{}/bin/aws", home),
            ];
            
            for path in user_paths {
                if std::path::Path::new(&path).exists() {
                    return Ok(path);
                }
            }
        }
        
        Err(anyhow!("AWS CLI not found. Please ensure AWS CLI is installed and in PATH, or set AWS_CLI_PATH environment variable to the full path of the aws command"))
    }

    /// Initialize AWS SDK config
    async fn init_aws_config(&self) -> Result<aws_config::SdkConfig> {
        let mut config_loader = aws_config::defaults(BehaviorVersion::latest());
        
        if let Some(profile) = &self.config.aws_profile {
            config_loader = config_loader.profile_name(profile);
        }
        
        Ok(config_loader.load().await)
    }

    /// Find bastion host by Name tag containing environment and "bastion"
    pub async fn find_bastion_instance(&self, aws_config: &aws_config::SdkConfig) -> Result<Instance> {
        let ec2_client = aws_sdk_ec2::Client::new(aws_config);
        
        let filters = vec![
            Filter::builder()
                .name("instance-state-name")
                .values("running")
                .build(),
        ];
        
        let resp = ec2_client
            .describe_instances()
            .set_filters(Some(filters))
            .send()
            .await?;
        
        // Find instance where Name tag contains both environment and "bastion" (case-insensitive)
        let env_lower = self.config.environment.to_lowercase();
        let instance = resp
            .reservations()
            .iter()
            .flat_map(|r| r.instances())
            .find(|i| {
                // Check Name tag
                for tag in i.tags() {
                    if tag.key() == Some("Name") {
                        if let Some(name) = tag.value() {
                            let name_lower = name.to_lowercase();
                            return name_lower.contains(&env_lower) && name_lower.contains("bastion");
                        }
                    }
                }
                false
            })
            .ok_or_else(|| anyhow!("No bastion instance found with name containing '{}' and 'bastion'", self.config.environment))?;
        
        // Log the found bastion for debugging
        for tag in instance.tags() {
            if tag.key() == Some("Name") {
                if let Some(name) = tag.value() {
                    eprintln!("Found bastion instance: {}", name);
                }
            }
        }
        
        if let Some(instance_id) = instance.instance_id() {
            eprintln!("Bastion instance ID: {}", instance_id);
        }
        
        Ok(instance.clone())
    }

    /// Get RDS endpoint for the environment
    pub async fn get_rds_endpoint(&self, aws_config: &aws_config::SdkConfig) -> Result<(String, u16)> {
        let rds_client = aws_sdk_rds::Client::new(aws_config);
        
        eprintln!("Listing all RDS instances...");
        let resp = rds_client
            .describe_db_instances()
            .send()
            .await?;
        
        // Log all found instances for debugging
        for db in resp.db_instances() {
            if let Some(id) = db.db_instance_identifier() {
                eprintln!("  Found RDS instance: {}", id);
            }
        }
        
        // Find RDS instance by environment or database name
        let env_lower = self.config.environment.to_lowercase();
        let db_name_lower = self.config.database_name.to_lowercase();
        
        let db_instance = resp
            .db_instances()
            .iter()
            .find(|db| {
                // Check if DB identifier contains environment name or database name
                if let Some(id) = db.db_instance_identifier() {
                    let id_lower = id.to_lowercase();
                    if id_lower.contains(&env_lower) || id_lower.contains(&db_name_lower) {
                        eprintln!("  Matched RDS instance: {}", id);
                        return true;
                    }
                }
                false
            })
            .ok_or_else(|| anyhow!("No RDS instance found containing '{}' or '{}' in identifier", 
                self.config.environment, self.config.database_name))?;
        
        let endpoint = db_instance
            .endpoint()
            .ok_or_else(|| anyhow!("RDS instance has no endpoint"))?;
        
        let host = endpoint
            .address()
            .ok_or_else(|| anyhow!("RDS endpoint has no address"))?;
        
        let port = endpoint.port().unwrap_or(5432) as u16;
        
        Ok((host.to_string(), port))
    }

    /// Establish SSH tunnel through bastion
    pub async fn establish_tunnel(&mut self) -> Result<u16> {
        eprintln!("Establishing SSH tunnel through bastion...");
        
        // Initialize AWS config
        eprintln!("Initializing AWS config...");
        let aws_config = match self.init_aws_config().await {
            Ok(config) => config,
            Err(e) => return Err(anyhow!("Failed to initialize AWS config: {}", e)),
        };
        
        // Find bastion instance
        eprintln!("Searching for bastion instance in environment '{}'...", self.config.environment);
        let bastion = match self.find_bastion_instance(&aws_config).await {
            Ok(instance) => instance,
            Err(e) => return Err(anyhow!("Failed to find bastion instance: {}", e)),
        };
        
        // Check if we should use instance ID or IP
        let bastion_instance_id = bastion.instance_id()
            .ok_or_else(|| anyhow!("Bastion has no instance ID"))?;
        
        let use_session_manager = self.config.use_session_manager ||
            bastion.public_ip_address().is_none() || 
            std::env::var("USE_SESSION_MANAGER").is_ok();
        
        let bastion_target = if use_session_manager {
            eprintln!("Using AWS Session Manager to connect to bastion");
            bastion_instance_id.to_string()
        } else {
            let ip = bastion.public_ip_address()
                .ok_or_else(|| anyhow!("Bastion has no public IP and Session Manager not configured"))?;
            eprintln!("Found bastion host IP: {}", ip);
            ip.to_string()
        };
        
        // Get RDS endpoint
        eprintln!("Searching for RDS instance...");
        let (rds_host, rds_port) = match self.get_rds_endpoint(&aws_config).await {
            Ok(endpoint) => endpoint,
            Err(e) => return Err(anyhow!("Failed to get RDS endpoint: {}", e)),
        };
        eprintln!("Found RDS endpoint: {}:{}", rds_host, rds_port);
        
        self.remote_host = Some(rds_host.clone());
        self.remote_port = rds_port;
        
        // Find available local port
        let local_port = Self::find_available_port()?;
        self.local_port = Some(local_port);
        
        // Build SSH command
        let mut ssh_cmd = Command::new("ssh");
        
        // Basic SSH options
        ssh_cmd
            .arg("-N") // No command execution
            .arg("-L")
            .arg(format!("{}:{}:{}", local_port, rds_host, rds_port));
        
        // Add connection target
        if use_session_manager {
            // When using Session Manager, just use the instance ID as host
            // The user's SSH config or our ProxyCommand will handle the connection
            ssh_cmd.arg(format!("{}@{}", self.config.bastion_user, bastion_instance_id));
            
            // Check if user has ProxyCommand in their SSH config for i-*
            let has_ssh_config_proxy = std::process::Command::new("ssh")
                .arg("-G")
                .arg(bastion_instance_id)
                .output()
                .ok()
                .and_then(|output| {
                    let config = String::from_utf8_lossy(&output.stdout);
                    if config.contains("proxycommand") && config.contains("ssm start-session") {
                        Some(true)
                    } else {
                        None
                    }
                })
                .unwrap_or(false);
            
            if !has_ssh_config_proxy {
                // User doesn't have ProxyCommand in SSH config, add it ourselves
                let aws_cmd = Self::find_aws_cli_path()?;
                eprintln!("Using AWS CLI at: {}", aws_cmd);
                
                ssh_cmd
                    .arg("-o")
                    .arg(format!("ProxyCommand={} ssm start-session --target {} --document-name AWS-StartSSHSession --parameters 'portNumber=%p'{}", 
                        aws_cmd,
                        bastion_instance_id,
                        if let Some(profile) = &self.config.aws_profile {
                            format!(" --profile {}", profile)
                        } else {
                            String::new()
                        }));
            } else {
                eprintln!("Using existing SSH config ProxyCommand for Session Manager");
                // Set AWS_PROFILE environment variable if specified
                if let Some(profile) = &self.config.aws_profile {
                    ssh_cmd.env("AWS_PROFILE", profile);
                }
            }
        } else {
            // Direct SSH connection
            ssh_cmd
                .arg(format!("{}@{}", self.config.bastion_user, bastion_target))
                .arg("-o")
                .arg("StrictHostKeyChecking=no")
                .arg("-o")
                .arg("UserKnownHostsFile=/dev/null");
        }
        
        // Common SSH options
        ssh_cmd
            .arg("-o")
            .arg("ServerAliveInterval=60")
            .arg("-o")
            .arg("ServerAliveCountMax=3")
            .arg("-o")
            .arg("ExitOnForwardFailure=yes")
            .arg("-o")
            .arg("ConnectTimeout=30");
        
        // Add SSH key if provided
        if let Some(key_path) = &self.config.ssh_key {
            // Convert to absolute path if relative
            let key_path = if std::path::Path::new(key_path).is_relative() {
                match std::env::current_dir() {
                    Ok(cwd) => cwd.join(key_path).to_string_lossy().to_string(),
                    Err(_) => key_path.clone(),
                }
            } else {
                key_path.clone()
            };
            
            // Verify key exists
            if !std::path::Path::new(&key_path).exists() {
                eprintln!("Warning: SSH key not found at: {}", key_path);
            }
            
            eprintln!("Using SSH key: {}", key_path);
            ssh_cmd.arg("-i").arg(&key_path);
            // For Session Manager, also disable other key attempts
            if use_session_manager {
                ssh_cmd
                    .arg("-o")
                    .arg("IdentitiesOnly=yes")
                    .arg("-o")
                    .arg("PreferredAuthentications=publickey");
            }
        } else if !use_session_manager {
            eprintln!("Using SSH agent for authentication");
            ssh_cmd.arg("-o").arg("PreferredAuthentications=publickey");
        }
        
        // Log the connection method
        if use_session_manager {
            eprintln!("SSH tunneling via Session Manager to instance: {}", bastion_instance_id);
        } else {
            eprintln!("SSH tunneling via direct connection to: {}@{}", self.config.bastion_user, bastion_target);
        }
        eprintln!("Local port forwarding: {} -> {}:{}", local_port, rds_host, rds_port);
        
        // Spawn SSH process
        eprintln!("Starting SSH tunnel on local port {}...", local_port);
        let mut child = match ssh_cmd
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn() {
            Ok(child) => child,
            Err(e) => return Err(anyhow!("Failed to spawn SSH process: {}", e)),
        };
        
        // Wait for tunnel to be established
        eprintln!("Waiting for tunnel to be ready...");
        let tunnel_ready = self.wait_for_tunnel(local_port).await;
        
        if tunnel_ready {
            // Detach stderr to prevent interference
            child.stderr.take();
            self.ssh_process = Some(child);
            eprintln!("SSH tunnel established successfully on port {}", local_port);
            Ok(local_port)
        } else {
            // Try to get stderr output for debugging
            if let Ok(Some(status)) = child.try_wait() {
                // Process already exited
                eprintln!("SSH process exited with status: {:?}", status);
                
                // Try to read stderr
                if let Some(stderr) = child.stderr.take() {
                    use tokio::io::AsyncReadExt;
                    let mut stderr_reader = tokio::io::BufReader::new(stderr);
                    let mut error_output = String::new();
                    if let Ok(_) = stderr_reader.read_to_string(&mut error_output).await {
                        // Only show first few lines of error output to avoid verbose SSH debug info
                        let lines: Vec<&str> = error_output.lines().take(10).collect();
                        eprintln!("SSH error output:\n{}", lines.join("\n"));
                    }
                }
                
                return Err(anyhow!("SSH process exited unexpectedly"));
            } else {
                eprintln!("SSH tunnel failed to establish after timeout");
                child.kill().await?;
            }
            Err(anyhow!("Failed to establish SSH tunnel - check SSH connectivity and credentials"))
        }
    }

    /// Wait for tunnel to be ready
    async fn wait_for_tunnel(&self, port: u16) -> bool {
        let max_attempts = 30;
        let delay = Duration::from_millis(500);
        
        for i in 0..max_attempts {
            if TcpListener::bind(format!("127.0.0.1:{}", port)).is_err() {
                // Port is now in use, tunnel might be ready
                if let Ok(Ok(_)) = timeout(
                    Duration::from_secs(1),
                    tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port))
                ).await {
                    return true;
                }
            }
            
            if i < max_attempts - 1 {
                sleep(delay).await;
            }
        }
        
        false
    }

    /// Get the connection string for the tunneled connection
    pub fn get_connection_string(&self, username: &str, password: &str, database: &str) -> Result<String> {
        let local_port = self.local_port
            .ok_or_else(|| anyhow!("Tunnel not established"))?;
        
        Ok(format!(
            "postgresql://{}:{}@localhost:{}/{}?sslmode=require",
            username, password, local_port, database
        ))
    }

    /// Health check for the tunnel
    pub async fn health_check(&self) -> bool {
        if let Some(port) = self.local_port {
            if let Ok(Ok(_)) = timeout(
                Duration::from_secs(1),
                tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port))
            ).await {
                return true;
            }
        }
        false
    }

    /// Cleanup tunnel on drop
    pub async fn cleanup(&mut self) -> Result<()> {
        if let Some(mut child) = self.ssh_process.take() {
            eprintln!("Cleaning up SSH tunnel...");
            child.kill().await?;
            child.wait().await?;
        }
        Ok(())
    }
}

impl Drop for TunnelManager {
    fn drop(&mut self) {
        if let Some(mut child) = self.ssh_process.take() {
            // Try to kill the process
            let _ = child.start_kill();
        }
    }
}