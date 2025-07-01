# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Query Crafter is a TUI (Terminal User Interface) application for interacting with databases, built with Rust using Ratatui. It supports both PostgreSQL and SQLite databases and provides an interactive terminal interface for querying and exploring database contents.

## Key Architecture

- **Component-based architecture**: Uses a trait-based component system where each UI element implements the `Component` trait
- **Async/await throughout**: Built on Tokio runtime with async database operations using SQLx
- **Database abstraction**: Uses the `Queryer` trait to abstract over PostgreSQL and SQLite connections
- **Event-driven**: Uses mpsc channels for action handling between components
- **TUI framework**: Built with Ratatui for terminal rendering

### Core Components

- `App` - Main application orchestrator that manages components and database connections
- `Db` - Primary database component handling table browsing, queries, and results display
- Component trait system with event handling, key bindings, and rendering
- Database abstraction layer (`sql.rs`) supporting both PostgreSQL and SQLite

### Configuration

- Database connections are configured in `config.toml` with multiple connection profiles
- Configuration is embedded in the binary at build time using `include_bytes!`
- Runtime configuration is handled through the `Config` struct

## Development Commands

### Build and Run
```bash
cargo build                                    # Debug build
cargo build --release                          # Release build
cargo run                                      # Connect to PostgreSQL using config.toml
cargo run -- <sqlite_file>                    # Connect to SQLite database file
```

### Testing and Quality
```bash
cargo test --all-features --workspace          # Run test suite
cargo fmt --all --check                        # Check formatting
cargo fmt --all                                # Format code
cargo clippy --all-targets --all-features --workspace -- -D warnings  # Lint check (strict)
cargo doc --no-deps --document-private-items --all-features --workspace  # Generate docs
```

## Recent Updates (Latest)

### Dependencies Updated
- **Ratatui**: Updated to 0.28.x with modern widget patterns
- **Tokio**: Updated to 1.45.x with latest async improvements  
- **SQLx**: Updated to 0.8.x with enhanced database support
- **Crossterm**: Updated to 0.28.x for better terminal handling

### Code Modernization
- Replaced deprecated `tui-popup` with native Ratatui modal dialogs
- Updated async patterns to use `fetch_all()` instead of manual iteration
- Fixed all clippy warnings for better code quality
- Modernized error handling and async/await patterns
- Removed deprecated `tokio-timer` dependency

### Database Setup
The application expects database connection details in `config.toml`. For development, ensure you have either:
- A PostgreSQL instance running with credentials matching `config.toml`
- A SQLite database file to pass as a command line argument

## Key Files

- `src/app.rs` - Main application logic and database connection handling
- `src/components/db.rs` - Database browser component with query execution
- `src/sql.rs` - Database abstraction layer with `Queryer` trait
- `src/tui.rs` - Terminal UI setup and event loop management
- `config.toml` - Database connection configurations (embedded at build time)

## Database Support

- **PostgreSQL**: Full support with connection pooling via SQLx
- **SQLite**: File-based database support for local development
- Database selection: PostgreSQL by default, SQLite when filename argument provided

## Component Architecture

Components implement the `Component` trait with methods for:
- Event handling (keyboard/mouse)
- State updates via Actions
- Rendering with Ratatui
- Configuration management

The main application uses an event loop that distributes events to components and processes actions through mpsc channels.