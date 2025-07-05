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

### Quick Install (Recommended)

Install the latest release using our installer script:

```bash
curl -sSfL https://raw.githubusercontent.com/JonnyWalker81/query-crafter/main/install.sh | sh
```

The installer will:
- Detect your operating system and architecture
- Download the appropriate binary from GitHub releases
- Install to `~/.local/bin` (or custom location via `INSTALL_DIR`)
- Verify checksums for security

### Installation Methods

#### 1. Universal Installer Script

```bash
# Install latest version
curl -sSfL https://raw.githubusercontent.com/JonnyWalker81/query-crafter/main/install.sh | sh

# Install specific version
curl -sSfL https://raw.githubusercontent.com/JonnyWalker81/query-crafter/main/install.sh | sh -s -- --version v0.1.0

# Install to custom location
INSTALL_DIR=/opt/bin curl -sSfL https://raw.githubusercontent.com/JonnyWalker81/query-crafter/main/install.sh | sh

# System-wide installation (requires sudo)
curl -sSfL https://raw.githubusercontent.com/JonnyWalker81/query-crafter/main/install.sh | sudo sh
```

#### 2. GitHub Releases

Download pre-built binaries directly from [GitHub Releases](https://github.com/JonnyWalker81/query-crafter/releases):

- Linux: `query-crafter-{version}-linux-{arch}.tar.gz`
- macOS: `query-crafter-{version}-macos-{arch}.tar.gz`
- Windows: `query-crafter-{version}-windows-{arch}.tar.gz`

Where `{arch}` is one of: `x86_64`, `arm64`, `armv7`, `i686`

#### 3. Cargo (Build from source)

```bash
cargo install query-crafter
```

#### 4. NixOS / Nix Package Manager

```bash
# Using nix flakes (recommended)
nix run github:JonnyWalker81/query-crafter

# Install to profile
nix profile install github:JonnyWalker81/query-crafter

# Traditional nix-build
curl -sSfL https://raw.githubusercontent.com/JonnyWalker81/query-crafter/main/nix/query-crafter.nix -o query-crafter.nix
nix-build query-crafter.nix
./result/bin/query-crafter

# In NixOS configuration.nix
environment.systemPackages = with pkgs; [
  (callPackage ./query-crafter.nix { })
];

# Development shell
nix develop github:JonnyWalker81/query-crafter
```

### Post-Installation

After installation, ensure the binary is in your PATH:

```bash
# Add to ~/.bashrc or ~/.zshrc if not already in PATH
export PATH="$HOME/.local/bin:$PATH"
```

Verify installation:

```bash
query-crafter --version
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
