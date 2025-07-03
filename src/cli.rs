use clap::Parser;

use crate::utils::version;

#[derive(Parser, Debug)]
#[command(author, version = version(), about)]
pub struct Cli {
  // Performance tuning options
  #[arg(short, long, value_name = "FLOAT", help = "Tick rate, i.e. number of ticks per second", default_value_t = 1.0)]
  pub tick_rate: f64,

  #[arg(
    short('r'),
    long,
    value_name = "FLOAT",
    help = "Frame rate, i.e. number of frames per second",
    default_value_t = 4.0
  )]
  pub frame_rate: f64,

  // Database connection options
  #[arg(short('H'), long = "host", value_name = "HOST", help = "Database server host or socket directory")]
  pub host: Option<String>,

  #[arg(short('p'), long = "port", value_name = "PORT", help = "Database server port")]
  pub port: Option<u16>,

  #[arg(short('U'), long = "username", value_name = "USERNAME", help = "Database user name")]
  pub username: Option<String>,

  #[arg(short('d'), long = "dbname", value_name = "DBNAME", help = "Database name to connect to")]
  pub dbname: Option<String>,

  #[arg(long = "password", help = "Force password prompt (should happen automatically)")]
  pub password_prompt: bool,

  #[arg(long = "connection-string", value_name = "CONNECTION_STRING", help = "Full PostgreSQL connection string")]
  pub connection_string: Option<String>,

  #[arg(
    long = "sslmode",
    value_name = "SSLMODE",
    help = "SSL mode (disable, allow, prefer, require, verify-ca, verify-full)"
  )]
  pub sslmode: Option<String>,

  #[arg(
    short('c'),
    long = "config-profile",
    value_name = "PROFILE",
    help = "Use specific connection profile from config.toml (0-based index)"
  )]
  pub config_profile: Option<usize>,

  // SQLite option
  #[arg(short('f'), long = "file", value_name = "FILE", help = "SQLite database file to use")]
  pub filename: Option<String>,

  // Positional database name (psql compatibility)
  #[arg(value_name = "DBNAME", help = "Database name (if not specified with -d)")]
  pub database: Option<String>,

  // SSH Tunnel options
  #[arg(long = "tunnel", help = "Enable SSH tunneling through AWS bastion host")]
  pub tunnel: bool,

  #[arg(short('e'), long = "env", value_name = "ENVIRONMENT", help = "AWS environment (dev, staging, production, etc.)")]
  pub environment: Option<String>,

  #[arg(long = "aws-profile", value_name = "PROFILE", help = "AWS profile to use (defaults to environment)")]
  pub aws_profile: Option<String>,

  #[arg(long = "bastion-user", value_name = "USER", help = "SSH user for bastion host", default_value = "ec2-user")]
  pub bastion_user: String,

  #[arg(long = "ssh-key", value_name = "PATH", help = "Path to SSH private key (uses ssh-agent by default)")]
  pub ssh_key: Option<String>,

  #[arg(long = "use-session-manager", help = "Force use of AWS Session Manager for SSH connection")]
  pub use_session_manager: bool,
}

impl Cli {
  /// Get the database name, preferring -d/--dbname over positional argument
  pub fn get_database_name(&self) -> Option<&String> {
    self.dbname.as_ref().or(self.database.as_ref())
  }

  /// Check if SQLite mode is requested
  pub fn is_sqlite_mode(&self) -> bool {
    self.filename.is_some()
  }

  /// Check if any PostgreSQL connection parameters are specified
  pub fn has_pg_connection_params(&self) -> bool {
    self.host.is_some()
      || self.port.is_some()
      || self.username.is_some()
      || self.get_database_name().is_some()
      || self.connection_string.is_some()
      || self.config_profile.is_some()
  }

  /// Check if tunnel mode is requested
  pub fn is_tunnel_mode(&self) -> bool {
    self.tunnel
  }

  /// Build PostgreSQL connection string from CLI args, environment variables, and config
  pub fn build_pg_connection_string(&self, config_connections: &[toml::Value]) -> Result<String, String> {
    // If full connection string provided, use it directly
    if let Some(conn_str) = &self.connection_string {
      return Ok(conn_str.clone());
    }

    // Determine which config profile to use
    let profile_index = self.config_profile.unwrap_or(0);
    let config_conn =
      config_connections.get(profile_index).ok_or_else(|| format!("Config profile {} not found", profile_index))?;

    // Get environment variables
    let env_host = std::env::var("PGHOST").ok();
    let env_port = std::env::var("PGPORT").ok().and_then(|s| s.parse().ok());
    let env_user = std::env::var("PGUSER").ok();
    let env_database = std::env::var("PGDATABASE").ok();
    let env_password = std::env::var("PGPASSWORD").ok();

    // Build connection parameters with CLI > ENV > CONFIG priority
    let host = self
      .host
      .as_ref()
      .map(|s| s.as_str())
      .or_else(|| env_host.as_deref())
      .or_else(|| config_conn["host"].as_str())
      .unwrap_or("localhost");

    let port = self.port.or(env_port).or_else(|| config_conn["port"].as_integer().map(|i| i as u16)).unwrap_or(5432);

    let username = self
      .username
      .as_ref()
      .map(|s| s.as_str())
      .or_else(|| env_user.as_deref())
      .or_else(|| config_conn["username"].as_str())
      .unwrap_or("postgres");

    let database = self
      .get_database_name()
      .map(|s| s.as_str())
      .or_else(|| env_database.as_deref())
      .or_else(|| config_conn["database"].as_str())
      .unwrap_or("postgres");

    let password = if self.password_prompt {
      // Force password prompt
      eprintln!("Password required for user '{}'", username);
      Self::prompt_password_with_paste_support()
    } else {
      // Check if any CLI parameters were provided (indicating user wants custom connection)
      let using_cli_params =
        self.host.is_some() || self.port.is_some() || self.username.is_some() || self.get_database_name().is_some();

      // Try to get password from environment or config
      let env_password_available = env_password.is_some();
      let config_password = if using_cli_params && !env_password_available {
        // When using CLI params without env password, don't fall back to config password
        None
      } else {
        config_conn["password"].as_str().map(|s| s.to_string())
      };

      let existing_password = env_password.or(config_password);

      match existing_password {
        Some(pwd) if !pwd.is_empty() => pwd,
        _ => {
          // No password available, prompt for it
          eprintln!("No password found in environment or config for user '{}'", username);
          Self::prompt_password_with_paste_support()
        },
      }
    };

    // Get SSL mode with priority: CLI > ENV > CONFIG > default
    let sslmode = self
      .sslmode
      .as_ref()
      .map(|s| s.clone())
      .or_else(|| std::env::var("PGSSLMODE").ok())
      .or_else(|| config_conn["sslmode"].as_str().map(|s| s.to_string()))
      .unwrap_or_else(|| "prefer".to_string());

    // Validate SSL mode
    let valid_sslmodes = ["disable", "allow", "prefer", "require", "verify-ca", "verify-full"];
    if !valid_sslmodes.contains(&sslmode.as_str()) {
      return Err(format!("Invalid SSL mode '{}'. Valid options: {}", sslmode, valid_sslmodes.join(", ")));
    }

    // Build connection string
    let connection_string = if password.is_empty() {
      format!("postgresql://{}@{}:{}/{}?sslmode={}", username, host, port, database, sslmode)
    } else {
      format!("postgresql://{}:{}@{}:{}/{}?sslmode={}", username, password, host, port, database, sslmode)
    };

    eprintln!("Using connection parameters:");
    eprintln!("  Host: {}", host);
    eprintln!("  Port: {}", port);
    eprintln!("  Username: {}", username);
    eprintln!("  Database: {}", database);
    eprintln!("  SSL Mode: {}", sslmode);
    eprintln!("  Password: {}", if password.is_empty() { "No" } else { "Yes" });

    Ok(connection_string)
  }

  /// Prompt for password with better paste support
  pub fn prompt_password_with_paste_support() -> String {
    use dialoguer::Password;

    // Try dialoguer first (better paste support)
    match Password::new().with_prompt("Password").allow_empty_password(false).interact() {
      Ok(password) => password,
      Err(_) => {
        // Fallback to rpassword if dialoguer fails
        eprintln!("Primary password input failed, trying fallback...");
        eprintln!("Tip: Use Ctrl+Shift+V or right-click to paste in most terminals");

        rpassword::prompt_password("Password (fallback): ").unwrap_or_else(|_| {
          eprintln!(
            "All password input methods failed. Please set PGPASSWORD environment variable or use --connection-string."
          );
          String::new()
        })
      },
    }
  }
}
