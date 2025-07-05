# Fix: SSH Tunnel Connection Crash

## Problem
When using SSH tunnel mode with command-line arguments, query-crafter crashes with:
```
The application panicked (crashed).
Message:  index not found
Location: src/app.rs:234
```

## Root Cause
The `connect_via_tunnel` function was using direct array indexing `app_config["connections"]` which panics when the "connections" key doesn't exist in config.toml. This happens when users provide all connection parameters via CLI arguments without a config file.

## Solution
Changed the code to use safe access:
```rust
// Before (crashes if "connections" key missing):
let connections = app_config["connections"].as_array();

// After (returns None if missing):
let connections = app_config.get("connections").and_then(|c| c.as_array());
```

## Testing
The tunnel connection should now work without config.toml:

```bash
PGPASSWORD=secret query-crafter \
  --tunnel \
  --env production \
  --aws-profile production \
  --bastion-user ec2-user \
  --use-session-manager \
  --ssh-key ../path/to/key.pem \
  --dbname mydb \
  --username myuser
```

This fix ensures both direct connections and tunnel connections work consistently without requiring config.toml when all parameters are provided via CLI.