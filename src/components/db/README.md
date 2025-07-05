# Database Component Module Structure

This module contains the main database interaction component, refactored from a single 3000+ line file into a modular structure.

## Module Organization

- **`mod.rs`** - Main module file containing the `Db` struct and `Component` trait implementation
- **`models.rs`** - Data structures (QueryHistoryEntry, DbColumn, DbTable, SelectionMode)
- **`editor.rs`** - EditorBackend enum and implementation for text editing functionality
- **`rendering.rs`** - All UI rendering functions (draw, render_*)
- **`handlers.rs`** - Event and keyboard input handling
- **`state.rs`** - State management and Action processing
- **`helpers.rs`** - Utility functions (query history, CSV export, autocomplete, etc.)

## Key Features

- **Database Table Browser** - Navigate and search database tables
- **Query Editor** - SQL query editing with vim-style keybindings
- **Query History** - Persistent query history with execution metadata
- **Results Viewer** - View query results with multiple display modes
- **Autocomplete** - SQL autocomplete with builtin and LSP backends
- **CSV Export** - Export query results to CSV files
- **Loading Indicators** - Visual feedback during query execution

## Public API

The main public interface is through the `Db` struct which implements the `Component` trait.
Commonly used types like `DbColumn` and `DbTable` are re-exported at the module level.