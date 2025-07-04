# query-crafter

[![CI](https://github.com//query-crafter/workflows/CI/badge.svg)](https://github.com//query-crafter/actions)

TUI for interacting with a database

## Features

- Interactive SQL query editor with VIM keybindings
- Database table browser
- Query result viewer with export capabilities
- SQL Language Server Protocol (LSP) support for intelligent autocomplete
- Support for PostgreSQL and SQLite databases
- SSH tunneling through AWS bastion hosts

## Installation

```bash
cargo install query-crafter
```

## Autocomplete

Query Crafter provides intelligent SQL autocomplete with two backends:

### Builtin Autocomplete (Recommended)
The builtin autocomplete uses your active database connection to provide accurate table and column suggestions. This is the recommended option, especially for SSH tunneling.

```toml
[autocomplete]
backend = "builtin"
```

### LSP Autocomplete (Experimental)
SQL Language Server Protocol support for enhanced SQL intelligence:

1. Install sql-language-server:
   ```bash
   npm install -g sql-language-server
   ```

2. Enable LSP in `~/.config/query-crafter/config.toml`:
   ```toml
   [autocomplete]
   backend = "lsp"  # or "hybrid" for both
   
   [lsp]
   enabled = true
   ```

**Note**: LSP autocomplete may have limited functionality with SSH tunneling. Use builtin autocomplete for best results with tunneled connections.

See [docs/LSP_SETUP.md](docs/LSP_SETUP.md) for detailed setup instructions.
