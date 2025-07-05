#!/bin/sh

# Query Crafter Universal Installer Script
# This script downloads and installs query-crafter from GitHub releases
# Usage: curl -sSfL https://raw.githubusercontent.com/JonnyWalker81/query-crafter/main/install.sh | sh

set -e

# Configuration
GITHUB_REPO="JonnyWalker81/query-crafter"
BINARY_NAME="query-crafter"
DEFAULT_INSTALL_DIR="$HOME/.local/bin"

# Colors for output
if [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    BLUE='\033[0;34m'
    BOLD='\033[1m'
    RESET='\033[0m'
else
    RED=''
    GREEN=''
    YELLOW=''
    BLUE=''
    BOLD=''
    RESET=''
fi

# Helper functions
info() {
    printf "${BLUE}info${RESET}: %s\n" "$1"
}

success() {
    printf "${GREEN}success${RESET}: %s\n" "$1"
}

warning() {
    printf "${YELLOW}warning${RESET}: %s\n" "$1"
}

error() {
    printf "${RED}error${RESET}: %s\n" "$1" >&2
}

die() {
    error "$1"
    exit 1
}

# Detect OS
detect_os() {
    OS=$(uname -s)
    case "$OS" in
        Linux*)
            # Check for specific Linux distributions
            if [ -f /etc/os-release ]; then
                . /etc/os-release
                if [ "$ID" = "nixos" ]; then
                    OS="nixos"
                else
                    OS="linux"
                fi
            else
                OS="linux"
            fi
            ;;
        Darwin*)
            OS="macos"
            ;;
        MINGW*|MSYS*|CYGWIN*)
            OS="windows"
            ;;
        FreeBSD)
            OS="freebsd"
            ;;
        *)
            die "Unsupported operating system: $OS"
            ;;
    esac
    echo "$OS"
}

# Detect architecture
detect_arch() {
    ARCH=$(uname -m)
    case "$ARCH" in
        x86_64|amd64)
            ARCH="x86_64"
            ;;
        aarch64|arm64)
            ARCH="arm64"
            ;;
        armv7l|armv7)
            ARCH="armv7"
            ;;
        i686|i386)
            ARCH="i686"
            ;;
        *)
            die "Unsupported architecture: $ARCH"
            ;;
    esac
    echo "$ARCH"
}

# Get latest release version from GitHub
get_latest_version() {
    VERSION_URL="https://api.github.com/repos/$GITHUB_REPO/releases/latest"
    
    if command -v curl >/dev/null 2>&1; then
        VERSION=$(curl -sSf "$VERSION_URL" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    elif command -v wget >/dev/null 2>&1; then
        VERSION=$(wget -qO- "$VERSION_URL" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    else
        die "Neither curl nor wget found. Please install one of them."
    fi
    
    if [ -z "$VERSION" ]; then
        die "Failed to get latest version from GitHub"
    fi
    
    echo "$VERSION"
}

# Construct download URL
get_download_url() {
    local version="$1"
    local os="$2"
    local arch="$3"
    
    # Map our OS/arch to the release naming convention
    case "$os" in
        linux)
            os_name="linux"
            ;;
        macos)
            os_name="macos"
            ;;
        windows)
            os_name="windows"
            ;;
        freebsd)
            os_name="freebsd"
            ;;
        nixos)
            # Special case - we'll handle this separately
            return 1
            ;;
    esac
    
    # Adjust architecture naming to match releases
    case "$arch" in
        arm64|aarch64)
            arch_name="arm64"
            ;;
        x86_64)
            arch_name="x86_64"
            ;;
        i686)
            arch_name="i686"
            ;;
        armv7)
            arch_name="armv7"
            ;;
    esac
    
    # Check for unsupported combinations
    if [ "$os_name" = "linux" ] && [ "$arch_name" != "x86_64" ]; then
        error "Pre-built binaries for Linux $arch_name are not available."
        echo ""
        echo "ARM Linux users should build from source:"
        echo "  cargo install query-crafter"
        echo ""
        echo "Or use the development environment:"
        echo "  git clone https://github.com/$GITHUB_REPO.git"
        echo "  cd query-crafter"
        echo "  cargo build --release"
        return 1
    fi
    
    echo "https://github.com/$GITHUB_REPO/releases/download/$version/$BINARY_NAME-$version-$os_name-$arch_name.tar.gz"
}

# Download file
download() {
    local url="$1"
    local output="$2"
    
    if command -v curl >/dev/null 2>&1; then
        curl -sSfL --retry 5 --retry-delay 2 "$url" -o "$output"
    elif command -v wget >/dev/null 2>&1; then
        wget -q --tries=5 --timeout=20 "$url" -O "$output"
    else
        die "Neither curl nor wget found. Please install one of them."
    fi
}

# Verify checksum
verify_checksum() {
    local file="$1"
    local checksum_url="$2"
    local temp_checksum="$file.sha256"
    
    info "Downloading checksum..."
    download "$checksum_url" "$temp_checksum" || {
        warning "Could not download checksum file. Skipping verification."
        rm -f "$temp_checksum"
        return 0
    }
    
    info "Verifying checksum..."
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum -c "$temp_checksum" >/dev/null 2>&1 || die "Checksum verification failed"
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 -c "$temp_checksum" >/dev/null 2>&1 || die "Checksum verification failed"
    else
        warning "No checksum tool found. Skipping verification."
    fi
    
    rm -f "$temp_checksum"
    success "Checksum verified"
}

# Install binary
install_binary() {
    local archive="$1"
    local install_dir="$2"
    
    # Create install directory if it doesn't exist
    if [ ! -d "$install_dir" ]; then
        info "Creating directory: $install_dir"
        mkdir -p "$install_dir" || die "Failed to create install directory"
    fi
    
    # Extract binary
    info "Extracting $BINARY_NAME..."
    tar -xzf "$archive" -C "$install_dir" || die "Failed to extract archive"
    
    # Make binary executable
    chmod +x "$install_dir/$BINARY_NAME" || die "Failed to make binary executable"
    
    success "Installed $BINARY_NAME to $install_dir"
}

# Check if directory is in PATH
check_path() {
    local dir="$1"
    case ":$PATH:" in
        *:"$dir":*)
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

# NixOS special handling
handle_nixos() {
    error "NixOS detected. Direct binary installation is not recommended on NixOS."
    echo ""
    echo "Please use one of these methods instead:"
    echo ""
    echo "1. Using nix-shell (temporary):"
    echo "   nix-shell -p query-crafter"
    echo ""
    echo "2. Add to configuration.nix (permanent):"
    echo "   environment.systemPackages = with pkgs; [ query-crafter ];"
    echo ""
    echo "3. Using home-manager:"
    echo "   home.packages = with pkgs; [ query-crafter ];"
    echo ""
    echo "4. Build from source:"
    echo "   git clone https://github.com/$GITHUB_REPO.git"
    echo "   cd query-crafter"
    echo "   nix-build"
    echo ""
    echo "Note: If query-crafter is not yet in nixpkgs, you can use the provided nix expression:"
    echo "   curl -sSfL https://raw.githubusercontent.com/$GITHUB_REPO/main/nix/query-crafter.nix -o query-crafter.nix"
    echo "   nix-build query-crafter.nix"
    exit 1
}

# Main installation function
main() {
    echo "${BOLD}Query Crafter Installer${RESET}"
    echo ""
    
    # Parse command line arguments
    VERSION=""
    while [ $# -gt 0 ]; do
        case "$1" in
            --version|-v)
                shift
                VERSION="$1"
                ;;
            --install-dir|-d)
                shift
                INSTALL_DIR="$1"
                ;;
            --help|-h)
                echo "Usage: $0 [OPTIONS]"
                echo ""
                echo "Options:"
                echo "  --version, -v <VERSION>    Install specific version (e.g., v0.2.0)"
                echo "  --install-dir, -d <DIR>    Install to specific directory"
                echo "  --help, -h                 Show this help message"
                exit 0
                ;;
            *)
                die "Unknown option: $1"
                ;;
        esac
        shift
    done
    
    # Detect OS and architecture
    OS=$(detect_os)
    ARCH=$(detect_arch)
    
    info "Detected OS: $OS"
    info "Detected architecture: $ARCH"
    
    # Special handling for NixOS
    if [ "$OS" = "nixos" ]; then
        handle_nixos
    fi
    
    # Get version if not specified
    if [ -z "$VERSION" ]; then
        info "Getting latest version..."
        VERSION=$(get_latest_version)
    fi
    info "Installing version: $VERSION"
    
    # Set install directory if not specified
    if [ -z "$INSTALL_DIR" ]; then
        INSTALL_DIR="$DEFAULT_INSTALL_DIR"
    fi
    
    # Get download URL
    DOWNLOAD_URL=$(get_download_url "$VERSION" "$OS" "$ARCH")
    if [ $? -ne 0 ]; then
        die "Failed to construct download URL"
    fi
    
    CHECKSUM_URL="$DOWNLOAD_URL.sha256"
    
    info "Download URL: $DOWNLOAD_URL"
    
    # Create temporary directory
    TEMP_DIR=$(mktemp -d)
    trap "rm -rf $TEMP_DIR" EXIT
    
    # Download archive
    ARCHIVE="$TEMP_DIR/$BINARY_NAME.tar.gz"
    info "Downloading $BINARY_NAME..."
    download "$DOWNLOAD_URL" "$ARCHIVE" || die "Failed to download $BINARY_NAME"
    
    # Verify checksum
    verify_checksum "$ARCHIVE" "$CHECKSUM_URL"
    
    # Install binary
    install_binary "$ARCHIVE" "$INSTALL_DIR"
    
    # Check PATH
    if ! check_path "$INSTALL_DIR"; then
        warning "$INSTALL_DIR is not in your PATH"
        echo ""
        echo "Add the following to your shell configuration file (.bashrc, .zshrc, etc.):"
        echo ""
        echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
        echo ""
    else
        success "$INSTALL_DIR is already in your PATH"
    fi
    
    # Verify installation
    if [ -x "$INSTALL_DIR/$BINARY_NAME" ]; then
        echo ""
        success "Installation complete!"
        echo ""
        echo "Run '$BINARY_NAME --help' to get started"
    else
        die "Installation verification failed"
    fi
}

# Run main function
main "$@"