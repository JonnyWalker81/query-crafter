use std::{
  thread,
  time::{Duration, Instant},
};

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
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
  tui,
};

pub struct App {
  pub config: Config,
  pub tick_rate: f64,
  pub frame_rate: f64,
  pub components: Vec<Box<dyn Component>>,
  pub should_quit: bool,
  pub should_suspend: bool,
  pub mode: Mode,
  pub last_tick_key_events: Vec<KeyEvent>,
  pool: sqlx::Pool<sqlx::Postgres>,
}

fn to_connection(config: &str) -> Result<String> {
  let app_config_contents = std::fs::read_to_string("config.toml")?;
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
  pub async fn new(tick_rate: f64, frame_rate: f64) -> Result<Self> {
    // let home = Home::new();
    // let fps = FpsCounter::default();
    let db = Db::new();
    let config = Config::new()?;
    let mode = Mode::Home;
    let connection = to_connection("config.toml")?;
    let pool = PgPoolOptions::new().max_connections(5).connect(&connection).await?;

    Ok(Self {
      tick_rate,
      frame_rate,
      // components: vec![Box::new(home), Box::new(fps)],
      components: vec![Box::new(db)],
      should_quit: false,
      should_suspend: false,
      config,
      mode,
      last_tick_key_events: Vec::new(),
      pool,
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

    init(action_tx.clone(), self.pool.clone())?;

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
            query(&q, action_tx.clone(), self.pool.clone()).await?;
          },
          Action::LoadTables => {
            // println!("Load Tables");
            load_tables(&self.pool, action_tx.clone()).await?;
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
            if let Err(e) = query(q, action_tx.clone(), self.pool.clone()).await {
              println!("Error executing query: {:?}", e);
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

async fn dispatch(tx: tokio::sync::mpsc::UnboundedSender<Action>, action: Action) -> Result<()> {
  if let Err(e) = tx.send(action) {
    println!("Error dipatching: {:?}", e);
  }

  Ok(())
}

async fn load_tables(pool: &sqlx::Pool<sqlx::Postgres>, tx: tokio::sync::mpsc::UnboundedSender<Action>) -> Result<()> {
  // println!("load_tabled called...");
  let mut rows =
    sqlx::query("SELECT * FROM information_schema.tables WHERE table_catalog = $1").bind("postgres").fetch(pool);

  let mut tables = Vec::new();
  while let Ok(Some(row)) = rows.try_next().await {
    let name: String = row.try_get("table_name").unwrap_or_default();
    let schema: String = row.try_get("table_schema").unwrap_or_default();
    // println!("table: {} ({})", name, schema);
    tables.push(DbTable { name, schema });
    // tables.push(Table {
    //     name: row.try_get("table_name")?,
    //     create_time: None,
    //     update_time: None,
    //     engine: None,
    //     schema: row.try_get("table_schema")?,
    // })
  }
  // println!("tables: {:?}", tables.len());

  tables.sort_by(|a, b| a.name.cmp(&b.name));
  dispatch(tx, Action::TablesLoaded(tables)).await?;

  Ok(())
}

fn init(tx: tokio::sync::mpsc::UnboundedSender<Action>, pool: sqlx::Pool<sqlx::Postgres>) -> Result<()> {
  // if let Some(tx) = &self.io_tx {
  //   Self::get_tables(self.pool.clone(), tx.clone()).await?
  // }
  tokio::spawn(async move {
    let pool = pool.clone();
    thread::sleep(Duration::from_millis(200));

    let _ = load_tables(&pool, tx).await;
    // if let Err(e) = tx.send(Action::LoadTables) {
    //   println!("Error sending load table event.");
    // }
  });
  Ok(())
}

async fn query(
  q: &str,
  tx: tokio::sync::mpsc::UnboundedSender<Action>,
  pool: sqlx::Pool<sqlx::Postgres>,
) -> Result<()> {
  let mut rows = sqlx::query(q).fetch(&pool);

  let mut headers = vec![];
  let mut results = vec![];
  while let Some(row) = rows.try_next().await? {
    if headers.len() == 0 {
      headers = row.columns().iter().map(|c| c.name().to_string()).collect();
    }
    let mut row_result = vec![];
    for c in row.columns() {
      if let Ok(v) = get_value(&row, c) {
        row_result.push(v);
      }
    }

    results.push(row_result);
  }

  dispatch(tx, Action::QueryResult(headers, results)).await?;
  Ok(())
}

#[macro_export]
macro_rules! get_or_null {
  ($value:expr) => {
    $value.map_or("NULL".to_string(), |v| v.to_string())
  };
}

fn get_value(row: &PgRow, column: &PgColumn) -> Result<String> {
  let column_name = column.name();
  if let Ok(value) = row.try_get(column_name) {
    let value: Option<i16> = value;
    let v = value.map_or("NULL".to_string(), |v| v.to_string());
    Ok(v)
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<i32> = value;
    let v = value.map_or("NULL".to_string(), |v| v.to_string());
    Ok(v)
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<i64> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<rust_decimal::Decimal> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: String = value;
    Ok(value)
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<NaiveDate> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: String = value;
    Ok(value)
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<chrono::DateTime<chrono::Utc>> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<chrono::DateTime<chrono::Local>> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<NaiveDateTime> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<NaiveDate> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<NaiveTime> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<serde_json::Value> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get::<Option<bool>, _>(column_name) {
    let value: Option<bool> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<Vec<String>> = value;
    Ok(value.map_or("NULL".to_string(), |v| v.join(",")))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<Uuid> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<&[u8]> = value;
    Ok(value.map_or("NULL".to_string(), |values| {
      format!("\\x{}", values.iter().map(|v| format!("{:02x}", v)).collect::<String>())
    }))
  } else {
    eyre::bail!("Unknown type for column {}", column_name);
  }
}
