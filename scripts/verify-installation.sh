#!/bin/sh

# Query Crafter Installation Verification Script
# This script checks if query-crafter is properly installed and configured

set -e

# Colors for output
if [ -t 1 ]; then
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    RED='\033[0;31m'
    RESET='\033[0m'
    BOLD='\033[1m'
else
    GREEN=''
    YELLOW=''
    RED=''
    RESET=''
    BOLD=''
fi

echo "${BOLD}Query Crafter Installation Verification${RESET}"
echo "======================================"
echo ""

# Check if query-crafter is in PATH
echo -n "Checking if query-crafter is in PATH... "
if command -v query-crafter >/dev/null 2>&1; then
    echo "${GREEN}✓ Found${RESET}"
    BINARY_PATH=$(command -v query-crafter)
    echo "  Location: $BINARY_PATH"
else
    echo "${RED}✗ Not found${RESET}"
    echo ""
    echo "query-crafter is not in your PATH. Possible locations:"
    echo "  - ~/.local/bin/query-crafter"
    echo "  - /usr/local/bin/query-crafter"
    echo "  - /opt/query-crafter/bin/query-crafter"
    echo ""
    echo "Add the installation directory to your PATH:"
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    exit 1
fi

# Check version
echo -n "Checking version... "
if VERSION=$(query-crafter --version 2>&1); then
    echo "${GREEN}✓${RESET} $VERSION"
else
    echo "${RED}✗ Failed to get version${RESET}"
    exit 1
fi

# Check configuration directory
echo -n "Checking configuration directory... "
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/query-crafter"
if [ -d "$CONFIG_DIR" ]; then
    echo "${GREEN}✓ Exists${RESET}"
    echo "  Location: $CONFIG_DIR"
else
    echo "${YELLOW}! Not found${RESET}"
    echo "  Creating configuration directory..."
    mkdir -p "$CONFIG_DIR"
    echo "  ${GREEN}✓ Created${RESET}: $CONFIG_DIR"
fi

# Check for config file
echo -n "Checking configuration file... "
CONFIG_FILE="$CONFIG_DIR/config.toml"
if [ -f "$CONFIG_FILE" ]; then
    echo "${GREEN}✓ Found${RESET}"
else
    echo "${YELLOW}! Not found${RESET}"
    echo "  Default configuration will be used"
    echo "  To create a custom config: cp /path/to/example/config.toml $CONFIG_FILE"
fi

# Check for SQL Language Server (optional)
echo -n "Checking for SQL Language Server (optional)... "
if command -v sql-language-server >/dev/null 2>&1; then
    echo "${GREEN}✓ Found${RESET}"
    LSP_VERSION=$(sql-language-server --version 2>&1 || echo "unknown")
    echo "  Version: $LSP_VERSION"
else
    echo "${YELLOW}! Not found${RESET}"
    echo "  SQL LSP provides enhanced autocomplete"
    echo "  Install with: npm install -g sql-language-server"
fi

# Check data directory
echo -n "Checking data directory... "
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/query-crafter"
if [ -d "$DATA_DIR" ]; then
    echo "${GREEN}✓ Exists${RESET}"
    echo "  Location: $DATA_DIR"
else
    echo "${YELLOW}! Not found${RESET}"
    echo "  Will be created on first use"
fi

# System information
echo ""
echo "${BOLD}System Information${RESET}"
echo "=================="
echo "OS: $(uname -s)"
echo "Architecture: $(uname -m)"
echo "Shell: $SHELL"

# Summary
echo ""
echo "${BOLD}Summary${RESET}"
echo "======="
if command -v query-crafter >/dev/null 2>&1; then
    echo "${GREEN}✓${RESET} Query Crafter is installed and ready to use!"
    echo ""
    echo "To get started:"
    echo "  query-crafter --help           # Show help"
    echo "  query-crafter                  # Start with config file"
    echo "  query-crafter -H host -d db    # Connect to PostgreSQL"
    echo "  query-crafter database.db      # Open SQLite database"
else
    echo "${RED}✗${RESET} Installation verification failed"
    echo "Please check the error messages above"
    exit 1
fi