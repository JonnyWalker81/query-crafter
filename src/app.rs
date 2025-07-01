use std::{
  any::Any,
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
    custom_vim_editor::CustomVimEditor,
    db::{Db, DbTable},
    fps::FpsCounter,
    home::Home,
    vim::Vim,
    Component, ComponentKind,
  },
  config::Config,
  editor_component::EditorComponent,
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
  pool: Option<sqlx::Pool<sqlx::Postgres>>,
  db: Arc<dyn Queryer>,
  pub editor: Box<dyn EditorComponent>,
}

static CONFIG: &[u8] = include_bytes!("../config.toml");

fn to_connection(config: &str) -> Result<String> {
  let app_config_contents = std::str::from_utf8(CONFIG)?;
  let app_config = toml::from_str::<Value>(app_config_contents)?;
  let v = app_config["connections"][0]["host"].clone();
  let host = app_config["connections"][0]["host"].as_str().map_or("localhost", |v| v);
  let port = app_config["connections"][0]["port"].as_integer().unwrap_or(5432);
  let username = app_config["connections"][0]["username"].as_str().map_or("postgres", |v| v);
  let password = app_config["connections"][0]["password"].as_str().map_or("", |v| v);
  let database = app_config["connections"][0]["database"].as_str().map_or("postgres", |v| v);
  let connection = format!("postgresql://{username}:{password}@{host}:{port}/{database}?sslmode=disable");
  println!("Connection: {connection}");

  Ok(connection)
}

impl App {
  pub async fn new(tick_rate: f64, frame_rate: f64, filename: Option<String>) -> Result<Self> {
    // let home = Home::new();
    // let fps = FpsCounter::default();
    let db = Db::new();
    let config = Config::new()?;
    let mode = Mode::Home;
    let (db_conn, pool) = match &filename {
      Some(f) => {
        let sqlite_conn = Arc::new(crate::sql::Sqlite::new(f).await?);
        (sqlite_conn as Arc<dyn Queryer>, None)
      },
      None => {
        let connection = to_connection("config.toml")?;
        let pool = PgPoolOptions::new().max_connections(5).connect(&connection).await?;
        let pg_conn = Arc::new(crate::sql::Postgres::new(&connection).await?);
        (pg_conn as Arc<dyn Queryer>, Some(pool))
      },
    };
    // let postgres = crate::sql::Postgres::new(&connection).await?;

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
      editor: Box::new(Vim::new(crate::editor_common::Mode::Normal)),
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
    self.editor.init(Rect::default())?;

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

            if let Some(action) = self.editor.on_key_event(key)? {
              action_tx.send(action)?;
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
            let current_text = self.editor.get_text();
            if self.editor.as_any().is::<Vim>() {
              self.editor = Box::new(CustomVimEditor::default());
            } else {
              self.editor = Box::new(Vim::new(crate::editor_common::Mode::Normal));
            }
            self.editor.set_text(&current_text);
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
