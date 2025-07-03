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
cargo build                                    # Debug build (tui-textarea editor only)
cargo build --features zep-editor              # Build with optional Zep editor support
cargo build --release                          # Release build

# PostgreSQL connections
cargo run                                      # Connect using config.toml (first profile)
cargo run -- -H host -p port -U user -d db    # Connect using CLI arguments (prompts for password)
cargo run -- --connection-string "postgresql://user:pass@host:port/db"  # Full connection string
cargo run -- -c 1                             # Use specific config.toml profile (0-based index)
cargo run -- --password                       # Force password prompt (supports pasting)
cargo run -- --sslmode disable               # Set SSL mode (disable, allow, prefer, require, verify-ca, verify-full)

# Password input methods (in order of preference):
# 1. Environment variable: PGPASSWORD=secret cargo run -- [args]
# 2. Interactive prompt with paste support (Ctrl+Shift+V or right-click)
# 3. Connection string: --connection-string "postgresql://user:pass@host/db"

# SQLite connections  
cargo run -- <sqlite_file>                    # Connect to SQLite database file (positional)
cargo run -- -f <sqlite_file>                 # Connect to SQLite database file (-f flag)

# Development
cargo run --example editor_demo                # Test editor backend switching
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

### Editor Configuration
Query Crafter supports multiple text editor backends for enhanced VIM editing:

- **Default (tui-textarea)**: Built-in VIM emulation using tui_textarea crate
- **Zep (optional)**: Advanced VIM editor with full C++ Zep integration

To switch editors, update `config.toml`:
```toml
[editor]
backend = "tui-textarea"  # or "zep"
```

### Zep Editor Setup
To enable the Zep editor backend:

1. **Install Dependencies**: Ensure C++ compiler and CMake are available
2. **Enable Feature**: Build with `cargo build --features zep-editor`
3. **Configure**: Set `backend = "zep"` in `config.toml`

Note: Zep editor requires ImGui integration and is currently in experimental status.

### Autocomplete Feature
Query Crafter includes intelligent SQL autocomplete functionality:

- **Manual Trigger**: Press `Ctrl+Space` while in insert mode to show autocomplete suggestions
- **Context-Aware**: Suggests table names, column names, and SQL keywords based on cursor position
- **Navigation**: Use `Tab/Down` and `Shift+Tab/Up` to navigate suggestions
- **Selection**: Press `Enter` to apply selected suggestion or `Esc` to dismiss
- **Smart Filtering**: Fuzzy matching with relevance scoring

### Database Setup
The application expects database connection details in `config.toml`. For development, ensure you have either:
- A PostgreSQL instance running with credentials matching `config.toml`
- A SQLite database file to pass as a command line argument

## Key Files

- `src/app.rs` - Main application logic and database connection handling
- `src/components/db.rs` - Database browser component with query execution and editor backend selection
- `src/components/vim.rs` - Default VIM editor implementation using tui_textarea
- `src/components/zep_editor.rs` - Optional Zep editor integration with C++ FFI
- `src/editor_component.rs` - Editor component trait abstraction
- `src/sql.rs` - Database abstraction layer with `Queryer` trait
- `src/tui.rs` - Terminal UI setup and event loop management
- `config.toml` - Database and editor configurations (embedded at build time)
- `build.rs` - Build script for compiling Zep C++ library (when feature enabled)
- `examples/editor_demo.rs` - Demo of editor backend switching functionality

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