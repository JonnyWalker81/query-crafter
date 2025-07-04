#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

pub mod action;
pub mod app;
pub mod autocomplete;
pub mod autocomplete_engine;
pub mod autocomplete_widget;
pub mod cli;
pub mod components;
pub mod config;
pub mod editor_common;
pub mod editor_component;
pub mod lsp;
pub mod lsp_patches;
pub mod mode;
pub mod patch_lsp;
pub mod sql;
pub mod tui;
pub mod tunnel;
pub mod utils;

use clap::Parser;
use cli::Cli;
use color_eyre::eyre::Result;

use crate::{
  app::App,
  utils::{initialize_logging, initialize_panic_handler, version},
};

async fn tokio_main() -> Result<()> {
  initialize_logging()?;

  initialize_panic_handler()?;

  let args = Cli::parse();
  
  // Handle --patch-lsp flag
  if args.patch_lsp {
    if let Err(e) = patch_lsp::patch_sql_language_server() {
      eprintln!("Error patching sql-language-server: {}", e);
      std::process::exit(1);
    }
    return Ok(());
  }
  
  let mut app = App::new(args.tick_rate, args.frame_rate, &args).await?;
  app.run().await?;

  Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
  if let Err(e) = tokio_main().await {
    eprintln!("{} error: Something went wrong", env!("CARGO_PKG_NAME"));
    Err(e)
  } else {
    Ok(())
  }
}
