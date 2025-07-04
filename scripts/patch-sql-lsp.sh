#!/bin/bash
# Script to patch sql-language-server after installation

set -e

echo "Checking for sql-language-server installation..."

# Try to find sql-language-server
SQL_LSP_PATH=""

# Check global npm installation
if command -v sql-language-server &> /dev/null; then
    SQL_LSP_BIN=$(which sql-language-server)
    # Follow symlinks to find the actual installation
    SQL_LSP_REAL=$(readlink -f "$SQL_LSP_BIN" 2>/dev/null || realpath "$SQL_LSP_BIN" 2>/dev/null || echo "$SQL_LSP_BIN")
    SQL_LSP_PATH=$(dirname "$SQL_LSP_REAL")/../lib/node_modules/sql-language-server
fi

# Check local node_modules
if [ -z "$SQL_LSP_PATH" ] && [ -d "node_modules/sql-language-server" ]; then
    SQL_LSP_PATH="node_modules/sql-language-server"
fi

# Check npm prefix
if [ -z "$SQL_LSP_PATH" ]; then
    NPM_PREFIX=$(npm config get prefix 2>/dev/null || echo "")
    if [ -n "$NPM_PREFIX" ] && [ -d "$NPM_PREFIX/lib/node_modules/sql-language-server" ]; then
        SQL_LSP_PATH="$NPM_PREFIX/lib/node_modules/sql-language-server"
    fi
fi

if [ -z "$SQL_LSP_PATH" ] || [ ! -d "$SQL_LSP_PATH" ]; then
    echo "sql-language-server not found. Please install it with:"
    echo "  npm install -g sql-language-server"
    exit 0
fi

echo "Found sql-language-server at: $SQL_LSP_PATH"

# Apply patches
PATCH_DIR="$(dirname "$0")/../lsp-patches"

# Patch complete.js
if [ -f "$SQL_LSP_PATH/dist/src/complete/complete.js" ]; then
    echo "Patching complete.js to remove console.time() debug output..."
    cp "$SQL_LSP_PATH/dist/src/complete/complete.js" "$SQL_LSP_PATH/dist/src/complete/complete.js.bak" 2>/dev/null || true
    patch -N -s "$SQL_LSP_PATH/dist/src/complete/complete.js" < "$PATCH_DIR/complete.js.patch" || true
fi

# Patch initializeLogging.js
if [ -f "$SQL_LSP_PATH/dist/src/initializeLogging.js" ]; then
    echo "Patching initializeLogging.js to fix log level..."
    cp "$SQL_LSP_PATH/dist/src/initializeLogging.js" "$SQL_LSP_PATH/dist/src/initializeLogging.js.bak" 2>/dev/null || true
    patch -N -s "$SQL_LSP_PATH/dist/src/initializeLogging.js" < "$PATCH_DIR/initializeLogging.js.patch" || true
fi

# Patch SettingStore.js
if [ -f "$SQL_LSP_PATH/dist/src/SettingStore.js" ]; then
    echo "Patching SettingStore.js to fix config handling..."
    cp "$SQL_LSP_PATH/dist/src/SettingStore.js" "$SQL_LSP_PATH/dist/src/SettingStore.js.bak" 2>/dev/null || true
    patch -N -s "$SQL_LSP_PATH/dist/src/SettingStore.js" < "$PATCH_DIR/SettingStore.js.patch" || true
fi

echo "Patches applied successfully!"