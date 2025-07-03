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
};

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
}

static CONFIG: &[u8] = include_bytes!("../config.toml");

impl App {
  pub async fn new(tick_rate: f64, frame_rate: f64, cli_args: &crate::cli::Cli) -> Result<Self> {
    // let home = Home::new();
    // let fps = FpsCounter::default();
    let config = Config::new()?;
    eprintln!("Config: {config:?}");
    let db = Db::new_with_config(Some(config.clone()));
    let mode = Mode::Home;
    let db_conn = if cli_args.is_sqlite_mode() {
      // SQLite mode - using -f/--file flag
      let filename = cli_args.filename.as_ref().unwrap();
      eprintln!("Connecting to SQLite database: {}", filename);
      let sqlite_conn = Arc::new(crate::sql::Sqlite::new(filename).await?);
      sqlite_conn as Arc<dyn Queryer>
    } else if let Some(db_name) = cli_args.get_database_name() {
      // Check if database name looks like a file path (contains .db, .sqlite, or path separators)
      if db_name.contains(".db") || db_name.contains(".sqlite") || db_name.contains("/") || db_name.contains("\\") {
        eprintln!("Detected SQLite database file from positional argument: {}", db_name);
        let sqlite_conn = Arc::new(crate::sql::Sqlite::new(db_name).await?);
        sqlite_conn as Arc<dyn Queryer>
      } else {
        // PostgreSQL mode with database name
        let app_config_contents = std::str::from_utf8(CONFIG)?;
        let app_config = toml::from_str::<toml::Value>(app_config_contents)?;
        let connections =
          app_config["connections"].as_array().ok_or_else(|| anyhow!("No connections found in config.toml"))?;

        let connection = cli_args
          .build_pg_connection_string(connections)
          .map_err(|e| anyhow!("Failed to build connection string: {}", e))?;

        eprintln!("Connecting to PostgreSQL: {}", connection);

        let _pool = PgPoolOptions::new().max_connections(5).connect(&connection).await?;
        let pg_conn = Arc::new(crate::sql::Postgres::new(&connection).await?);
        pg_conn as Arc<dyn Queryer>
      }
    } else {
      // Default PostgreSQL mode - use CLI args, environment variables, and config.toml
      let app_config_contents = std::str::from_utf8(CONFIG)?;
      let app_config = toml::from_str::<toml::Value>(app_config_contents)?;
      let connections =
        app_config["connections"].as_array().ok_or_else(|| anyhow!("No connections found in config.toml"))?;

      let connection = cli_args
        .build_pg_connection_string(connections)
        .map_err(|e| anyhow!("Failed to build connection string: {}", e))?;

      eprintln!("Connecting to PostgreSQL: {}", connection);

      let _pool = PgPoolOptions::new().max_connections(5).connect(&connection).await?;
      let pg_conn = Arc::new(crate::sql::Postgres::new(&connection).await?);
      pg_conn as Arc<dyn Queryer>
    };
    // let postgres = crate::sql::Postgres::new(&connection).await?;

    Ok(Self {
      tick_rate,
      frame_rate,
      filename: cli_args.filename.clone(),
      // components: vec![Box::new(home), Box::new(fps)],
      components: vec![Box::new(db)],
      should_quit: false,
      should_suspend: false,
      config,
      mode,
      last_tick_key_events: Vec::new(),
      db: db_conn,
    })
  }

  pub async fn run(&mut self) -> Result<()> {
    let (action_tx, mut action_rx) = mpsc::unbounded_channel();

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

    init(action_tx.clone(), self.db.clone()).await?;

    loop {
      if let Some(e) = tui.next().await {
        match e {
          tui::Event::Quit => action_tx.send(Action::Quit)?,
          tui::Event::Tick => action_tx.send(Action::Tick)?,
          tui::Event::Render => action_tx.send(Action::Render)?,
          tui::Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
          tui::Event::Key(key) => {
            if let Some(keymap) = self.config.keybindings.get(&self.mode) {
              if let Some(action) = keymap.get(&vec![key]) {
                log::info!("Got action: {action:?}");
                action_tx.send(action.clone())?;
              } else {
                // If the key was not handled as a single key action,
                // then consider it for multi-key combinations.
                self.last_tick_key_events.push(key);

                // Check for multi-key combinations
                if let Some(action) = keymap.get(&self.last_tick_key_events) {
                  log::info!("Got action: {action:?}");
                  action_tx.send(action.clone())?;
                }
              }
            };

            // Editor key events are now handled by the Db component's editor backend
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
            // println!("Load Table: {}", table_name);
            let q = format!("SELECT * from {table_name}");
            query(&q, action_tx.clone(), self.db.clone()).await?;
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
            if let Err(e) = query(q, action_tx.clone(), self.db.clone()).await {
              dispatch(action_tx.clone(), Action::Error(format!("Error executing query: {e:?}"))).await?;
            }
          },
          Action::SwitchEditor => {
            // Editor switching is now handled by the Db component's editor backend configuration
            // This could be implemented by updating the config and calling register_config_handler
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
