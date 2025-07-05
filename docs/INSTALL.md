# Query Crafter Installation Guide

This guide provides detailed installation instructions for Query Crafter across different operating systems and environments.

## Table of Contents

- [Quick Install](#quick-install)
- [Supported Platforms](#supported-platforms)
- [Installation Methods](#installation-methods)
  - [Universal Installer Script](#universal-installer-script)
  - [Manual Installation](#manual-installation)
  - [Package Managers](#package-managers)
  - [Building from Source](#building-from-source)
- [Platform-Specific Instructions](#platform-specific-instructions)
  - [Linux](#linux)
  - [macOS](#macos)
  - [Windows](#windows)
  - [NixOS](#nixos)
- [Post-Installation Setup](#post-installation-setup)
- [Troubleshooting](#troubleshooting)
- [Uninstallation](#uninstallation)

## Quick Install

The fastest way to install Query Crafter:

```bash
curl -sSfL https://raw.githubusercontent.com/JonnyWalker81/query-crafter/main/install.sh | sh
```

This will install the latest version to `~/.local/bin`.

## Supported Platforms

Query Crafter provides pre-built binaries for:

| Platform | Architectures |
|----------|--------------|
| Linux    | x86_64, aarch64 (ARM64), armv7, i686 |
| macOS    | x86_64 (Intel), aarch64 (Apple Silicon) |
| Windows  | x86_64 |

## Installation Methods

### Universal Installer Script

The installer script automatically detects your platform and downloads the correct binary.

#### Basic Usage

```bash
# Install latest version
curl -sSfL https://raw.githubusercontent.com/JonnyWalker81/query-crafter/main/install.sh | sh
```

#### Advanced Options

```bash
# Install specific version
curl -sSfL https://raw.githubusercontent.com/JonnyWalker81/query-crafter/main/install.sh | sh -s -- --version v0.1.0

# Install to custom directory
INSTALL_DIR=/opt/query-crafter curl -sSfL https://raw.githubusercontent.com/JonnyWalker81/query-crafter/main/install.sh | sh

# System-wide installation
curl -sSfL https://raw.githubusercontent.com/JonnyWalker81/query-crafter/main/install.sh | sudo sh

# View installer options
curl -sSfL https://raw.githubusercontent.com/JonnyWalker81/query-crafter/main/install.sh | sh -s -- --help
```

### Manual Installation

1. Download the appropriate archive from [GitHub Releases](https://github.com/JonnyWalker81/query-crafter/releases)
2. Extract the binary:
   ```bash
   tar -xzf query-crafter-v0.1.0-linux-x86_64.tar.gz
   ```
3. Move to a directory in your PATH:
   ```bash
   sudo mv query-crafter /usr/local/bin/
   # or for user installation
   mkdir -p ~/.local/bin
   mv query-crafter ~/.local/bin/
   ```
4. Make executable (if needed):
   ```bash
   chmod +x ~/.local/bin/query-crafter
   ```

### Package Managers

#### Cargo (Rust Package Manager)

Build and install from source:

```bash
cargo install query-crafter
```

Requirements:
- Rust toolchain (install from https://rustup.rs)
- C compiler
- OpenSSL development libraries

### Building from Source

Clone and build the repository:

```bash
# Clone repository
git clone https://github.com/JonnyWalker81/query-crafter.git
cd query-crafter

# Build release version
cargo build --release

# Binary will be at target/release/query-crafter
./target/release/query-crafter --version

# Install to PATH
cargo install --path .
```

## Platform-Specific Instructions

### Linux

#### Debian/Ubuntu

```bash
# Install dependencies for building from source
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev

# Install using script
curl -sSfL https://raw.githubusercontent.com/JonnyWalker81/query-crafter/main/install.sh | sh
```

#### Fedora/RHEL/CentOS

```bash
# Install dependencies for building from source
sudo dnf install -y gcc pkg-config openssl-devel

# Install using script
curl -sSfL https://raw.githubusercontent.com/JonnyWalker81/query-crafter/main/install.sh | sh
```

#### Arch Linux

```bash
# Install dependencies for building from source
sudo pacman -S base-devel pkg-config openssl

# Install using script
curl -sSfL https://raw.githubusercontent.com/JonnyWalker81/query-crafter/main/install.sh | sh
```

### macOS

```bash
# Install using Homebrew (when available)
# brew install query-crafter

# Or use the installer script
curl -sSfL https://raw.githubusercontent.com/JonnyWalker81/query-crafter/main/install.sh | sh
```

For Apple Silicon (M1/M2) Macs, the installer automatically downloads the ARM64 version.

### Windows

#### Option 1: Windows Subsystem for Linux (WSL)

Use the Linux installation instructions within WSL.

#### Option 2: Native Windows

1. Download the Windows binary from [GitHub Releases](https://github.com/JonnyWalker81/query-crafter/releases)
2. Extract `query-crafter.exe` from the archive
3. Add to PATH or move to a directory already in PATH

#### Option 3: Using PowerShell (Coming Soon)

```powershell
# PowerShell installer script (planned)
iwr -useb https://raw.githubusercontent.com/JonnyWalker81/query-crafter/main/install.ps1 | iex
```

### NixOS

NixOS requires special handling due to its unique filesystem layout.

#### Using Flakes (Recommended)

```bash
# Run directly
nix run github:JonnyWalker81/query-crafter

# Install to profile
nix profile install github:JonnyWalker81/query-crafter

# Add to configuration.nix
{
  environment.systemPackages = with pkgs; [
    (fetchFromGitHub {
      owner = "JonnyWalker81";
      repo = "query-crafter";
      # ... specify rev and sha256
    })
  ];
}
```

#### Using Traditional Nix

```bash
# Download and build
curl -sSfL https://raw.githubusercontent.com/JonnyWalker81/query-crafter/main/nix/query-crafter.nix -o query-crafter.nix
nix-build query-crafter.nix
./result/bin/query-crafter
```

#### Development Shell

```bash
# Enter development environment
nix develop github:JonnyWalker81/query-crafter

# Or using shell.nix
nix-shell https://raw.githubusercontent.com/JonnyWalker81/query-crafter/main/nix/shell.nix
```

## Post-Installation Setup

### 1. Add to PATH

If installed to `~/.local/bin`, ensure it's in your PATH:

```bash
# Add to ~/.bashrc, ~/.zshrc, or equivalent
export PATH="$HOME/.local/bin:$PATH"

# Reload shell configuration
source ~/.bashrc
```

### 2. Verify Installation

```bash
query-crafter --version
```

### 3. Configure Database Connections

Create configuration file:

```bash
mkdir -p ~/.config/query-crafter
cat > ~/.config/query-crafter/config.toml << 'EOF'
[database]
# Add your database configurations here

[autocomplete]
backend = "builtin"

[editor]
backend = "tui-textarea"
EOF
```

### 4. Install SQL Language Server (Optional)

For enhanced autocomplete:

```bash
npm install -g sql-language-server
```

## Troubleshooting

### Common Issues

#### "command not found" after installation

**Solution**: Add installation directory to PATH
```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

#### Permission denied when running installer

**Solution**: Ensure the install directory is writable
```bash
mkdir -p ~/.local/bin
# Or use sudo for system-wide installation
```

#### SSL/TLS errors on older systems

**Solution**: Update CA certificates
```bash
# Debian/Ubuntu
sudo apt update && sudo apt install ca-certificates

# RHEL/CentOS
sudo yum install ca-certificates
```

#### Binary doesn't run on Linux (GLIBC errors)

**Solution**: The binary requires GLIBC 2.17 or newer. Check your version:
```bash
ldd --version
```

For older systems, build from source or use a statically-linked version.

#### macOS "cannot be opened" security warning

**Solution**: Remove quarantine attribute
```bash
xattr -d com.apple.quarantine ~/.local/bin/query-crafter
```

Or allow in System Preferences > Security & Privacy.

### Getting Help

- Check existing issues: https://github.com/JonnyWalker81/query-crafter/issues
- Create a new issue with:
  - Operating system and version
  - Architecture (output of `uname -m`)
  - Installation method used
  - Complete error message

## Uninstallation

### Installed via Script

```bash
# Remove binary
rm -f ~/.local/bin/query-crafter

# Remove configuration (optional)
rm -rf ~/.config/query-crafter

# Remove cache/data (optional)
rm -rf ~/.local/share/query-crafter
```

### Installed via Cargo

```bash
cargo uninstall query-crafter
```

### Installed via Nix

```bash
# If installed to profile
nix profile remove query-crafter

# If using nix-env
nix-env -e query-crafter
```

## Version Management

To manage multiple versions:

1. Install to version-specific directories:
   ```bash
   INSTALL_DIR=~/.local/bin/query-crafter-v0.1.0 curl -sSfL ... | sh
   ```

2. Use symbolic links:
   ```bash
   ln -sf ~/.local/bin/query-crafter-v0.1.0/query-crafter ~/.local/bin/query-crafter
   ```

3. Or use a version manager (when available).

## Security

- All releases are signed with SHA256 checksums
- The installer verifies checksums before installation
- Binaries are built in CI/CD with reproducible builds
- Report security issues to: security@query-crafter.dev (update with actual email)

## Contributing

For development setup and contribution guidelines, see [CONTRIBUTING.md](../CONTRIBUTING.md).