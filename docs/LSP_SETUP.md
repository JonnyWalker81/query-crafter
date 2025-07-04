# LSP Autocomplete Setup

Query Crafter supports SQL Language Server Protocol (LSP) for intelligent autocomplete with database-aware suggestions.

## Installation

1. Install sql-language-server globally:
   ```bash
   npm install -g sql-language-server
   ```

2. After installing query-crafter, run the patch script:
   ```bash
   # If installed via cargo install
   ~/.cargo/bin/query-crafter --patch-lsp
   
   # Or run the script directly
   ./scripts/patch-sql-lsp.sh
   ```

   The patches fix known issues in sql-language-server:
   - Removes debug output that interferes with LSP protocol
   - Fixes logging configuration
   - Adds safety checks for configuration handling

## Configuration

1. Create a `.sqllsrc.json` file in your project root:
   ```json
   {
     "connections": [
       {
         "name": "default",
         "adapter": "postgres",
         "host": "localhost",
         "port": 5432,
         "user": "postgres",
         "password": "your_password",
         "database": "your_database",
         "projectPaths": ["/path/to/your/project"]
       }
     ]
   }
   ```

2. Enable LSP in your `~/.config/query-crafter/config.toml`:
   ```toml
   [autocomplete]
   backend = "lsp"  # or "hybrid" for both LSP and builtin

   [lsp]
   enabled = true
   ```

## Usage

1. Start query-crafter
2. In the query editor, type SQL and press `Ctrl+Space` to trigger autocomplete
3. The LSP will provide:
   - Table names from your connected database
   - Column names when typing after a table name
   - SQL keywords and functions

## Troubleshooting

If LSP autocomplete isn't working:

1. Check that sql-language-server is installed:
   ```bash
   which sql-language-server
   ```

2. Verify patches were applied:
   ```bash
   ./scripts/patch-sql-lsp.sh
   ```

3. Check LSP logs:
   ```bash
   tail -f /tmp/sql-language-server.log
   ```

4. Ensure your `.sqllsrc.json` is valid JSON and in the project root

5. Try the builtin sql-lsp-wrapper:
   - The wrapper is automatically used if found in the same directory as query-crafter
   - It filters out debug messages that can interfere with the LSP protocol

## Alternative: Using sql-lsp-wrapper

If patching doesn't work or you prefer not to modify the global installation, query-crafter includes a built-in wrapper that filters problematic output:

```toml
# In ~/.config/query-crafter/config.toml
[lsp]
enabled = true
server_command = "sql-lsp-wrapper"  # Uses the built-in wrapper
```

The wrapper is automatically installed alongside query-crafter when using `cargo install`.