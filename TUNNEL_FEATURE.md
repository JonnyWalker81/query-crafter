# AWS RDS Tunnel Feature

## Overview
Added SSH tunneling capability to connect to RDS instances through AWS bastion hosts while maintaining full backward compatibility with existing direct connections.

## New CLI Options
- `--tunnel`: Enable SSH tunneling through AWS bastion host
- `-e, --env <ENVIRONMENT>`: AWS environment (dev, staging, production, etc.)
- `--aws-profile <PROFILE>`: AWS profile to use (defaults to environment)
- `--bastion-user <USER>`: SSH user for bastion host (default: ec2-user)
- `--ssh-key <PATH>`: Path to SSH private key (uses ssh-agent by default)
- `--use-session-manager`: Force use of AWS Session Manager for SSH connection

## Usage Examples

### Direct connection (existing functionality)
```bash
# Using config.toml
cargo run

# Using CLI arguments
cargo run -- -H localhost -p 5432 -U user -d mydb

# SQLite
cargo run -- database.db
```

### Tunneled connection (new functionality)
```bash
# Basic tunnel with environment
cargo run -- --tunnel --env staging -d mydb

# With specific AWS profile
cargo run -- --tunnel --env production --aws-profile prod-profile -d mydb

# With custom SSH settings
cargo run -- --tunnel --env dev --bastion-user ubuntu --ssh-key ~/.ssh/custom_key -d mydb

# Force AWS Session Manager (for private bastions)
cargo run -- --tunnel --env staging --use-session-manager -d mydb
```

## How It Works

1. **AWS Resource Discovery**:
   - Uses AWS SDK to find bastion host by Name tag containing both the environment and "bastion" (case-insensitive)
   - Example: For `--env staging`, it would match instances named "staging-bastion", "bastion-staging-01", etc.
   - Queries RDS instances to find the database endpoint
   - Supports multiple environments: dev, staging, master-staging, production, feature

2. **SSH Tunnel Management**:
   - Supports two connection methods:
     - Direct SSH to bastion public IP (with SSH key)
     - AWS Session Manager (for private bastions or when preferred)
   - Automatically detects when to use Session Manager
   - Spawns system SSH process with port forwarding
   - Finds available local port automatically
   - Monitors tunnel health with timeouts
   - Cleans up tunnel on exit

3. **Connection Flow**:
   - Parse CLI arguments
   - If `--tunnel` flag is present:
     - Initialize AWS SDK with profile/credentials
     - Query EC2 for bastion instance
     - Query RDS for database endpoint
     - Establish SSH tunnel
     - Connect SQLx to localhost:local_port
   - Otherwise use existing direct connection logic

## Configuration

Added tunnel configuration section to `config.toml`:
```toml
# SSH Tunnel configuration
# The bastion search looks for EC2 instances where the Name tag contains both:
# - The environment name (e.g., "staging")
# - The word "bastion"
# Example: An instance named "staging-bastion" or "bastion-staging-01" would match for env=staging
[tunnel]
environments = ["dev", "staging", "master-staging", "production", "feature"]
default_user = "ec2-user"
```

## Implementation Details

- **New Module**: `src/tunnel.rs` contains `TunnelManager` struct
- **AWS Integration**: Uses `aws-sdk-ec2` and `aws-sdk-rds` for resource discovery
- **Process Management**: Uses `tokio::process::Command` to spawn SSH
- **Error Handling**: Comprehensive error messages with retry logic
- **Backward Compatibility**: All existing functionality remains unchanged

## Dependencies Added
- `aws-config = "1.1"`
- `aws-sdk-ec2 = "1.91"`
- `aws-sdk-rds = "1.90"`

## Security Considerations
- SSH host key checking is disabled for convenience (can be made configurable)
- Supports both SSH key files and ssh-agent
- Passwords are handled securely with existing prompt mechanisms
- Connection strings mask passwords in logs

## Troubleshooting

### AWS CLI not found error
If you get an error about AWS CLI not being found when using Session Manager, you can:

1. Set the `AWS_CLI_PATH` environment variable:
   ```bash
   export AWS_CLI_PATH=/path/to/aws
   cargo run -- --tunnel --env staging --use-session-manager -d mydb
   ```

2. Or find your AWS CLI path and set it:
   ```bash
   which aws  # Find the path
   export AWS_CLI_PATH=$(which aws)
   ```

3. For Nix users, the tool checks common Nix paths automatically, but you can also:
   ```bash
   export AWS_CLI_PATH=$HOME/.nix-profile/bin/aws
   ```