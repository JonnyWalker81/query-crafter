# SQL Language Server Protocol (LSP) Support

Query Crafter supports SQL Language Server Protocol for enhanced autocomplete functionality. This provides more accurate completions, syntax checking, and other IDE-like features.

## Installation

### 1. Install SQL Language Server

First, install a SQL language server. We recommend `sql-language-server`:

```bash
npm install -g sql-language-server
```

### 2. Configure Query Crafter

Update your `config.toml` to enable LSP:

```toml
[autocomplete]
backend = "lsp"  # or "hybrid" for both LSP and builtin

[lsp]
enabled = true
server_command = "sql-language-server"
```

## Configuration Options

### Autocomplete Backends

- **builtin**: Uses the built-in fuzzy matching autocomplete (default)
- **lsp**: Uses only the Language Server Protocol
- **hybrid**: Uses both LSP and builtin, preferring LSP when available

### LSP Configuration

```toml
[lsp]
enabled = true                          # Enable/disable LSP
server_name = "sql-language-server"     # Server identifier
server_command = "sql-language-server"  # Command to launch
server_args = []                        # Additional command arguments
trigger_characters = [".", " "]         # Characters that trigger completion
```

### SQL Language Server Configuration

Create a `.sqllsrc.json` file in your project root:

```json
{
  "connections": [
    {
      "name": "default",
      "adapter": "postgres",
      "host": "localhost",
      "port": 5432,
      "user": "postgres",
      "database": "mydb"
    }
  ]
}
```

## Features

When LSP is enabled, you get:

- **Context-aware completions**: Table names, column names, SQL keywords
- **Hover information**: Details about tables and columns
- **Signature help**: Function parameter hints
- **Diagnostics**: Syntax error detection (coming soon)

## Troubleshooting

### LSP Server Not Found

If you get an error about the LSP server not being found:

1. Ensure the server is installed globally
2. Check that it's in your PATH
3. Try using the full path in `server_command`

### No Completions Appearing

1. Check that LSP is enabled in your config
2. Verify the server is running (check logs)
3. Ensure your `.sqllsrc.json` is properly configured

### Performance Issues

If LSP completions are slow:

1. Try the "hybrid" backend to fall back to builtin when needed
2. Adjust the completion timeout in future versions
3. Check your language server's performance settings

## Supported Language Servers

Currently tested with:
- `sql-language-server` (recommended)

Other SQL language servers may work but are untested.

## Future Enhancements

- Support for multiple language servers
- Diagnostic error display in the editor
- Code formatting support
- Go to definition for tables/views
- Rename refactoring