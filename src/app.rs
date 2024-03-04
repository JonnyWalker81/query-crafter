use std::{
  sync::Arc,
  thread,
  time::{Duration, Instant},
};

use color_eyre::eyre::{self, anyhow, Result};
use crossterm::event::KeyEvent;
use ratatui::prelude::Rect;
use serde::{Deserialize, Serialize};
use sqlx::{
  postgres::{PgColumn, PgPoolOptions, PgRow},
  types::Uuid,
  Column, Postgres, Row,
};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use toml::Value;

use crate::{
  action::Action,
  components::{
    db::{Db, DbTable},
    fps::FpsCounter,
    home::Home,
    Component, ComponentKind,
  },
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
  pool: sqlx::Pool<sqlx::Postgres>,
  db: Arc<dyn Queryer>,
}

static CONFIG: &'static [u8] = include_bytes!("../config.toml");

fn to_connection(config: &str) -> Result<String> {
  let app_config_contents = std::str::from_utf8(CONFIG)?;
  let app_config = toml::from_str::<Value>(&app_config_contents)?;
  let v = app_config["connections"][0]["host"].clone();
  let host = app_config["connections"][0]["host"].as_str().map_or("localhost", |v| v);
  let _port = app_config["connections"][0]["port"].as_integer().unwrap_or(5432);
  let username = app_config["connections"][0]["username"].as_str().map_or("postgres", |v| v);
  let password = app_config["connections"][0]["password"].as_str().map_or("", |v| v);
  let database = app_config["connections"][0]["database"].as_str().map_or("postgres", |v| v);
  let connection = format!("postgres://{}:{}@{}/{}", username, password, host, database);

  Ok(connection)
}

impl App {
  pub async fn new(tick_rate: f64, frame_rate: f64, filename: Option<String>) -> Result<Self> {
    // let home = Home::new();
    // let fps = FpsCounter::default();
    let db = Db::new();
    let config = Config::new()?;
    let mode = Mode::Home;
    let connection = to_connection("config.toml")?;
    let pool = PgPoolOptions::new().max_connections(5).connect(&connection).await?;
    let db_conn: Arc<dyn Queryer> = match &filename {
      Some(f) => Arc::new(crate::sql::Sqlite::new(&f).await?),
      None => Arc::new(crate::sql::Postgres::new(&connection).await?),
    };
    let postgres = crate::sql::Postgres::new(&connection).await?;

    Ok(Self {
      tick_rate,
      frame_rate,
      filename,
      // components: vec![Box::new(home), Box::new(fps)],
      components: vec![Box::new(db)],
      should_quit: false,
      should_suspend: false,
      config,
      mode,
      last_tick_key_events: Vec::new(),
      pool,
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
      component.init(tui.size()?)?;
    }

    init(action_tx.clone(), self.db.clone())?;

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
                let r = component.draw(f, f.size());
                if let Err(e) = r {
                  action_tx.send(Action::Error(format!("Failed to draw: {:?}", e))).unwrap();
                }
              }
            })?;
          },
          Action::Render => {
            tui.draw(|f| {
              for component in self.components.iter_mut() {
                let r = component.draw(f, f.size());
                if let Err(e) = r {
                  action_tx.send(Action::Error(format!("Failed to draw: {:?}", e))).unwrap();
                }
              }
            })?;
          },
          Action::LoadTable(ref table_name) => {
            // println!("Load Table: {}", table_name);
            let q = format!("SELECT * from {}", table_name);
            query(&q, action_tx.clone(), self.db.clone()).await?;
          },
          Action::LoadTables(ref search) => {
            // println!("Load Tables");
            load_tables(&self.pool, action_tx.clone(), search).await?;
          },
          Action::SelectComponent(ref kind) => {
            match kind {
              ComponentKind::Home => {
                // println!("home mode");
                self.mode = Mode::Home;
              },
              ComponentKind::Query => {
                // println!("query mode");
                self.mode = Mode::Query;
              },
              ComponentKind::Results => {
                // println!("resuts mode");
                self.mode = Mode::Results;
              },
            }
          },
          Action::HandleQuery(ref q) => {
            // println!("Execute Query: {}", q);
            if let Err(e) = query(q, action_tx.clone(), self.db.clone()).await {
              // println!("Error executing query: {:?}", e);
              dispatch(action_tx.clone(), Action::Error(format!("Error executing query: {:?}", e))).await?;
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
    Ok(())
  }
}

pub async fn dispatch(tx: tokio::sync::mpsc::UnboundedSender<Action>, action: Action) -> Result<()> {
  if let Err(e) = tx.send(action) {
    println!("Error dipatching: {:?}", e);
  }

  Ok(())
}

async fn load_tables(
  pool: &sqlx::Pool<sqlx::Postgres>,
  tx: tokio::sync::mpsc::UnboundedSender<Action>,
  search: &str,
) -> Result<()> {
  let mut rows =
    sqlx::query("SELECT * FROM information_schema.tables WHERE table_catalog = $1").bind("postgres").fetch(pool);

  let mut tables = Vec::new();
  while let Ok(Some(row)) = rows.try_next().await {
    let name: String = row.try_get("table_name").unwrap_or_default();
    let schema: String = row.try_get("table_schema").unwrap_or_default();
    tables.push(DbTable { name, schema });
  }

  tables.sort_by(|a, b| a.name.cmp(&b.name));
  let t = if search.is_empty() { tables } else { tables.iter().filter(|t| t.name.contains(search)).cloned().collect() };

  dispatch(tx, Action::TablesLoaded(t)).await?;

  Ok(())
}

// fn init(tx: tokio::sync::mpsc::UnboundedSender<Action>, pool: sqlx::Pool<sqlx::Postgres>) -> Result<()> {
fn init(tx: tokio::sync::mpsc::UnboundedSender<Action>, db: Arc<dyn Queryer>) -> Result<()> {
  tokio::spawn(async move {
    // let pool = pool.clone();
    thread::sleep(Duration::from_millis(200));

    if let Err(e) = db.load_tables(tx, "").await {
      println!("Error sending load table event.");
    }
  });
  Ok(())
}

async fn query(q: &str, tx: tokio::sync::mpsc::UnboundedSender<Action>, db: Arc<dyn Queryer>) -> Result<()> {
  db.query(q, tx).await?;
  Ok(())
}
