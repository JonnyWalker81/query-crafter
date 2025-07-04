use std::sync::Arc;

use color_eyre::eyre::{anyhow, Result};
use crossterm::event::KeyEvent;
use ratatui::prelude::Rect;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::mpsc;

use crate::{
  action::Action,
  components::{db::Db, Component, ComponentKind},
  config::Config,
  mode::Mode,
  sql::Queryer,
  tui,
  tunnel::{TunnelConfig, TunnelManager},
};

const DEFAULT_CONFIG: &str = r#"# Editor configuration
[editor]
backend = "tui-textarea"

# Database connections
# Add your database connections here
# [[connections]]
# host = "localhost"
# port = 5432
# username = "postgres"
# password = "password"
# database = "postgres"
# sslmode = "prefer"
"#;

pub struct App {
  pub config: Config,
  pub tick_rate: f64,
  pub frame_rate: f64,
  pub filename: Option<String>,
  pub components: Vec<Box<dyn Component>>,
  pub should_quit: bool,
  pub should_suspend: bool,
  pub mode: Mode,
  pub last_tick_key_events: Vec<KeyEvent>,
  db: Arc<dyn Queryer>,
  tunnel_manager: Option<TunnelManager>,
  lsp_service: Option<crate::lsp::LspService>,
  connection_info: Option<ConnectionInfo>,
}

#[derive(Clone)]
struct ConnectionInfo {
  host: String,
  port: u16,
  username: String,
  password: String,
  database: String,
  adapter: String,
}

// Load config at runtime to prevent constant rebuilds
fn load_config_toml() -> Result<String> {
    // Try locations in order:
    // 1. Current directory (for development)
    if let Ok(content) = std::fs::read_to_string("config.toml") {
        return Ok(content);
    }
    
    // 2. User config directory
    if let Some(proj_dirs) = directories::ProjectDirs::from("com", "query-crafter", "query-crafter") {
        let config_dir = proj_dirs.config_dir();
        let config_path = config_dir.join("config.toml");
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            return Ok(content);
        }
        
        // Create config directory and default config if it doesn't exist
        if !config_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(config_dir) {
                eprintln!("Warning: Could not create config directory: {e}");
            } else {
                // Try to write default config
                if let Err(e) = std::fs::write(&config_path, DEFAULT_CONFIG) {
                    eprintln!("Warning: Could not create default config.toml: {e}");
                } else {
                    eprintln!("Created default config at: {}", config_path.display());
                    eprintln!("Please edit this file to add your database connections.");
                    return Ok(DEFAULT_CONFIG.to_string());
                }
            }
        }
    }
    
    // 3. System config directory
    if let Ok(content) = std::fs::read_to_string("/etc/query-crafter/config.toml") {
        return Ok(content);
    }
    
    // 4. Executable directory (for portable installs)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            if let Ok(content) = std::fs::read_to_string(exe_dir.join("config.toml")) {
                return Ok(content);
            }
        }
    }
    
    // 5. Return default config if none found
    Ok(DEFAULT_CONFIG.to_string())
}

impl App {
  pub async fn new(tick_rate: f64, frame_rate: f64, cli_args: &crate::cli::Cli) -> Result<Self> {
    let config = Config::new()?;
    let db = Db::new_with_config(Some(config.clone()));
    let mode = Mode::Home;

    let mut tunnel_manager = None;
    let mut connection_info = None;

    let db_conn = if cli_args.is_sqlite_mode() {
      // SQLite mode - using -f/--file flag
      let filename = cli_args.filename.as_ref().unwrap();
      eprintln!("Connecting to SQLite database: {filename}");
      let sqlite_conn = Arc::new(crate::sql::Sqlite::new(filename).await?);
      sqlite_conn as Arc<dyn Queryer>
    } else if let Some(db_name) = cli_args.get_database_name() {
      // Check if database name looks like a file path (contains .db, .sqlite, or path separators)
      if db_name.contains(".db") || db_name.contains(".sqlite") || db_name.contains("/") || db_name.contains("\\") {
        eprintln!("Detected SQLite database file from positional argument: {db_name}");
        let sqlite_conn = Arc::new(crate::sql::Sqlite::new(db_name).await?);
        sqlite_conn as Arc<dyn Queryer>
      } else {
        // PostgreSQL mode with database name
        if cli_args.is_tunnel_mode() {
          // Tunnel mode
          Self::connect_via_tunnel(cli_args, db_name, &mut tunnel_manager, &mut connection_info).await?
        } else {
          // Direct connection
          Self::connect_direct(cli_args).await?
        }
      }
    } else {
      // Default PostgreSQL mode
      if cli_args.is_tunnel_mode() {
        // Tunnel mode with default database
        let db_name = cli_args.get_database_name().map(|s| s.as_str()).unwrap_or("postgres");
        Self::connect_via_tunnel(cli_args, db_name, &mut tunnel_manager, &mut connection_info).await?
      } else {
        // Direct connection
        Self::connect_direct(cli_args).await?
      }
    };

    Ok(Self {
      tick_rate,
      frame_rate,
      filename: cli_args.filename.clone(),
      components: vec![Box::new(db)],
      should_quit: false,
      should_suspend: false,
      config,
      mode,
      last_tick_key_events: Vec::new(),
      db: db_conn,
      tunnel_manager,
      lsp_service: None, // Will be initialized in run() when we have action_tx
      connection_info, // Set during connection establishment
    })
  }

  /// Connect directly to PostgreSQL without tunnel
  async fn connect_direct(cli_args: &crate::cli::Cli) -> Result<Arc<dyn Queryer>> {
    let app_config_contents = load_config_toml()?;
    let app_config = toml::from_str::<toml::Value>(&app_config_contents)?;
    let connections = app_config.get("connections")
      .and_then(|c| c.as_array())
      .ok_or_else(|| {
        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "query-crafter", "query-crafter") {
          let config_path = proj_dirs.config_dir().join("config.toml");
          anyhow!("No database connections found in config.toml. Please add connections to: {}", config_path.display())
        } else {
          anyhow!("No database connections found in config.toml")
        }
      })?;

    let connection = cli_args
      .build_pg_connection_string(connections)
      .map_err(|e| anyhow!("Failed to build connection string: {e}"))?;

    eprintln!("Connecting to PostgreSQL: {connection}");

    let _pool = PgPoolOptions::new().max_connections(5).connect(&connection).await?;
    let pg_conn = Arc::new(crate::sql::Postgres::new(&connection).await?);
    Ok(pg_conn as Arc<dyn Queryer>)
  }

  /// Connect to PostgreSQL via SSH tunnel
  async fn connect_via_tunnel(
    cli_args: &crate::cli::Cli,
    database: &str,
    tunnel_manager: &mut Option<TunnelManager>,
    connection_info: &mut Option<ConnectionInfo>,
  ) -> Result<Arc<dyn Queryer>> {
    // Validate required parameters
    let environment =
      cli_args.environment.as_ref().ok_or_else(|| anyhow!("--env parameter is required when using --tunnel"))?;

    // Create tunnel config
    let tunnel_config = TunnelConfig {
      environment: environment.clone(),
      aws_profile: cli_args.aws_profile.clone(),
      bastion_user: cli_args.bastion_user.clone(),
      ssh_key: cli_args.ssh_key.clone(),
      database_name: database.to_string(),
      use_session_manager: cli_args.use_session_manager,
    };

    // Create and establish tunnel
    let mut tunnel = TunnelManager::new(tunnel_config);
    let _local_port = tunnel.establish_tunnel().await?;

    // Get connection parameters from config or CLI
    let app_config_contents = load_config_toml()?;
    let app_config = toml::from_str::<toml::Value>(&app_config_contents)?;
    let connections = app_config["connections"].as_array();

    // Build connection string for tunneled connection
    let env_user = std::env::var("PGUSER").ok();
    let username = cli_args
      .username
      .as_deref()
      .or(env_user.as_deref())
      .or_else(|| connections.and_then(|c| c.first()).and_then(|c| c["username"].as_str()))
      .unwrap_or("postgres");

    let password = if cli_args.password_prompt {
      eprintln!("Password required for user '{username}'");
      crate::cli::Cli::prompt_password_with_paste_support()
    } else {
      std::env::var("PGPASSWORD")
        .ok()
        .or_else(|| connections.and_then(|c| c.first()).and_then(|c| c["password"].as_str()).map(|s| s.to_string()))
        .unwrap_or_else(|| {
          eprintln!("No password found for user '{username}'");
          crate::cli::Cli::prompt_password_with_paste_support()
        })
    };

    let connection = tunnel.get_connection_string(username, &password, database)?;
    eprintln!("Connecting to PostgreSQL via tunnel: {}", connection.replace(&password, "***"));

    let _pool = PgPoolOptions::new().max_connections(5).connect(&connection).await?;
    let pg_conn = Arc::new(crate::sql::Postgres::new(&connection).await?);

    // Store connection info for LSP
    if let Some(local_port) = tunnel.get_local_port() {
      *connection_info = Some(ConnectionInfo {
        host: "localhost".to_string(),
        port: local_port,
        username: username.to_string(),
        password: password.clone(),
        database: database.to_string(),
        adapter: "postgres".to_string(),
      });
    }

    // Store tunnel manager to keep it alive
    *tunnel_manager = Some(tunnel);

    Ok(pg_conn as Arc<dyn Queryer>)
  }

  pub async fn run(&mut self) -> Result<()> {
    let (action_tx, mut action_rx) = mpsc::unbounded_channel();

    // Check if we're in tunnel mode
    let is_tunnel_mode = self.tunnel_manager.is_some();
    
    // Initialize LSP service if configured and not in tunnel mode
    if self.config.lsp.enabled && self.config.autocomplete.backend != "builtin" && !is_tunnel_mode {
      eprintln!("Initializing LSP service...");
      
      // Write database connection info for LSP if we have it
      if let Some(conn_info) = self.get_current_connection_info() {
        eprintln!("Writing LSP database configuration...");
        if let Err(e) = self.write_lsp_config(&conn_info).await {
          eprintln!("Warning: Failed to write LSP config: {}", e);
        } else {
          // Give sql-language-server time to potentially detect the new config
          tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
      }
      
      match crate::lsp::LspService::new(self.config.lsp.clone(), action_tx.clone()).await {
        Ok(service) => {
          eprintln!("LSP service initialized successfully");
          self.lsp_service = Some(service);
        }
        Err(e) => {
          eprintln!("Failed to initialize LSP service: {}. Falling back to builtin autocomplete.", e);
          // Continue without LSP
        }
      }
    } else if is_tunnel_mode && self.config.autocomplete.backend != "builtin" {
      eprintln!("SSH tunnel detected - using builtin autocomplete for better compatibility");
    }

    let mut tui = tui::Tui::new()?.tick_rate(self.tick_rate).frame_rate(self.frame_rate);
    // tui.mouse(true);
    tui.enter()?;

    for component in self.components.iter_mut() {
      component.register_action_handler(action_tx.clone())?;
    }

    for component in self.components.iter_mut() {
      component.register_config_handler(self.config.clone())?;
    }

    for component in self.components.iter_mut() {
      component.init(Rect::default())?;
    }
    
    // Notify components about tunnel mode
    if is_tunnel_mode {
      action_tx.send(Action::SetTunnelMode(true))?;
    }

    init(action_tx.clone(), self.db.clone()).await?;

    loop {
      if let Some(e) = tui.next().await {
        match e {
          tui::Event::Quit => action_tx.send(Action::Quit)?,
          tui::Event::Tick => action_tx.send(Action::Tick)?,
          tui::Event::Render => action_tx.send(Action::Render)?,
          tui::Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
          tui::Event::Key(key) => {
            let mut handled = false;
            if let Some(keymap) = self.config.keybindings.get(&self.mode) {
              if let Some(action) = keymap.get(&vec![key]) {
                log::info!("Got action: {action:?}");
                action_tx.send(action.clone())?;
                handled = true;
              } else {
                // If the key was not handled as a single key action,
                // then consider it for multi-key combinations.
                self.last_tick_key_events.push(key);

                // Check for multi-key combinations
                if let Some(action) = keymap.get(&self.last_tick_key_events) {
                  log::info!("Got action: {action:?}");
                  action_tx.send(action.clone())?;
                  handled = true;
                }
              }
            };

            // If the key was handled by keybindings, don't pass it to components
            if handled {
              continue;
            }
          },
          _ => {},
        }
        for component in self.components.iter_mut() {
          if let Some(action) = component.handle_events(Some(e.clone()))? {
            action_tx.send(action)?;
          }
        }
      }

      while let Ok(action) = action_rx.try_recv() {
        if action != Action::Tick && action != Action::Render {
          log::debug!("{action:?}");
        }
        match action {
          Action::Tick => {
            self.last_tick_key_events.drain(..);
          },
          Action::Quit => self.should_quit = true,
          Action::Suspend => self.should_suspend = true,
          Action::Resume => self.should_suspend = false,
          Action::Resize(w, h) => {
            tui.resize(Rect::new(0, 0, w, h))?;
            tui.draw(|f| {
              for component in self.components.iter_mut() {
                let r = component.draw(f, f.area());
                if let Err(e) = r {
                  action_tx.send(Action::Error(format!("Failed to draw: {e:?}"))).unwrap();
                }
              }
            })?;
          },
          Action::Render => {
            tui.draw(|f| {
              for component in self.components.iter_mut() {
                let r = component.draw(f, f.area());
                if let Err(e) = r {
                  action_tx.send(Action::Error(format!("Failed to draw: {e:?}"))).unwrap();
                }
              }
            })?;
          },
          Action::LoadTable(ref table_name) => {
            // Load columns for the table
            // Find the schema from the tables list in the first component (Db)
            let schema = "public";
            if let Some(_db_component) = self.components.first() {
              // This is a bit of a hack - we need access to the table schema
              // For now, assume public schema for PostgreSQL
              // For SQLite, the schema is always "public" as set in load_tables
            }
            load_table_columns(table_name, schema, action_tx.clone(), self.db.clone()).await?;
          },
          Action::LoadTables(ref search) => {
            // println!("Load Tables");
            self.db.load_tables(action_tx.clone(), search).await?;
          },
          Action::SelectComponent(ref kind) => {
            match kind {
              ComponentKind::Home => {
                self.mode = Mode::Home;
              },
              ComponentKind::Query => {
                self.mode = Mode::Query;
              },
              ComponentKind::Results => {
                self.mode = Mode::Results;
              },
            }
          },
          Action::HandleQuery(ref q) => {
            // Send query started notification
            dispatch(action_tx.clone(), Action::QueryStarted).await?;
            
            if let Err(e) = query(q, action_tx.clone(), self.db.clone()).await {
              dispatch(action_tx.clone(), Action::Error(format!("Error executing query: {e:?}"))).await?;
            }
          },
          Action::SwitchEditor => {
            // Editor switching is now handled by the Db component's editor backend configuration
            // This could be implemented by updating the config and calling register_config_handler
          },
          Action::RequestAutocomplete { ref text, cursor_line, cursor_col, ref context } => {
            if let Some(lsp_service) = &self.lsp_service {
              eprintln!("Processing LSP autocomplete request...");
              // Handle autocomplete request asynchronously
              if let Err(e) = lsp_service.handle_autocomplete_request(
                text.clone(),
                cursor_line,
                cursor_col,
                context.clone(),
              ).await {
                eprintln!("LSP autocomplete error: {}", e);
              }
            } else {
              eprintln!("LSP service not available for autocomplete");
            }
          },
          Action::UpdateAutocompleteDocument(ref text) => {
            if let Some(lsp_service) = &self.lsp_service {
              // Update document in LSP service
              if let Err(e) = lsp_service.update_document(text.clone()).await {
                eprintln!("Failed to update LSP document: {}", e);
              }
            }
          },
          _ => {},
        }
        for component in self.components.iter_mut() {
          if let Some(action) = component.update(action.clone())? {
            action_tx.send(action)?
          };
        }
      }

      if self.should_suspend {
        tui.suspend()?;
        action_tx.send(Action::Resume)?;
        tui = tui::Tui::new()?.tick_rate(self.tick_rate).frame_rate(self.frame_rate);
        // tui.mouse(true);
        tui.enter()?;
      } else if self.should_quit {
        tui.stop()?;
        break;
      }
    }
    tui.exit()?;

    // Cleanup tunnel if established
    if let Some(mut tunnel) = self.tunnel_manager.take() {
      tunnel.cleanup().await?;
    }

    Ok(())
  }
  
  /// Get current connection info if available
  fn get_current_connection_info(&self) -> Option<ConnectionInfo> {
    self.connection_info.clone()
  }
  
  /// Write LSP configuration with current database connection
  async fn write_lsp_config(&self, conn_info: &ConnectionInfo) -> Result<()> {
    use serde_json::json;
    
    let config = json!({
      "connections": [{
        "name": "query-crafter",
        "adapter": &conn_info.adapter,
        "host": &conn_info.host,
        "port": conn_info.port,
        "user": &conn_info.username,
        "password": &conn_info.password,
        "database": &conn_info.database,
        "projectPaths": [std::env::current_dir()?.to_string_lossy()]
      }]
    });
    
    // Write to .sqllsrc.json in current directory
    let config_path = std::env::current_dir()?.join(".sqllsrc.json");
    
    // Remove old config if exists to ensure fresh read
    if config_path.exists() {
      std::fs::remove_file(&config_path)?;
    }
    
    std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
    eprintln!("Wrote LSP config to: {:?}", config_path);
    eprintln!("LSP config content: {}", serde_json::to_string_pretty(&config)?);
    
    Ok(())
  }
}

pub async fn dispatch(tx: tokio::sync::mpsc::UnboundedSender<Action>, action: Action) -> Result<()> {
  if let Err(e) = tx.send(action) {
    println!("Error dipatching: {e:?}");
  }

  Ok(())
}

async fn init(tx: tokio::sync::mpsc::UnboundedSender<Action>, db: Arc<dyn Queryer>) -> Result<()> {
  if let Err(e) = db.load_tables(tx, "").await {
    eprintln!("Error loading tables: {e:?}");
  }
  Ok(())
}

async fn query(q: &str, tx: tokio::sync::mpsc::UnboundedSender<Action>, db: Arc<dyn Queryer>) -> Result<()> {
  db.query(q, tx).await?;
  Ok(())
}

async fn load_table_columns(
  table_name: &str,
  schema: &str,
  tx: tokio::sync::mpsc::UnboundedSender<Action>,
  db: Arc<dyn Queryer>,
) -> Result<()> {
  let columns = db.load_table_columns(table_name, schema).await?;
  dispatch(tx, Action::TableColumnsLoaded(table_name.to_string(), columns)).await?;
  Ok(())
}
