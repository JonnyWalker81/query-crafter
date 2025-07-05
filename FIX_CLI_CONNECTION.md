# Fix: CLI Connection Without config.toml

## Problem
When using command-line arguments to connect to a database, query-crafter was still requiring a config.toml file with at least one connection defined, even though all connection parameters were provided via CLI.

## Root Cause
The `connect_direct` function in `app.rs` was always requiring connections to be defined in config.toml, even when CLI arguments provided all necessary connection information.

## Solution
Modified the connection logic to:

1. **Check for CLI parameters**: Added logic to detect when CLI connection parameters are provided
2. **Allow empty config**: Modified `connect_direct` to allow empty connections array when CLI params exist
3. **Fix build_pg_connection_string**: Updated to handle cases where no config connections exist but CLI params do

## Changes Made

### src/app.rs
- Modified `connect_direct` to check if CLI has connection parameters
- Allow empty connections array when CLI params are provided
- Only show "No database connections found" error when both config is empty AND no CLI params exist

### src/cli.rs
- Updated `build_pg_connection_string` to handle empty config connections
- Use an empty TOML table as fallback when no config exists but CLI params are provided

## Testing
After these changes, you can now connect using CLI arguments without any config.toml:

```bash
PGPASSWORD=secret query-crafter \
  --host localhost \
  --port 5432 \
  --dbname postgres \
  --username postgres
```

Or with a connection string:
```bash
query-crafter --connection-string "postgresql://user:pass@localhost:5432/dbname"
```

## Config Directory
The application will create a default config.toml at first run in:
- macOS: `~/Library/Application Support/com.query-crafter.query-crafter/`
- Linux: `~/.config/query-crafter/`
- Windows: `%APPDATA%\query-crafter\query-crafter\`

But this is no longer required when using CLI arguments.