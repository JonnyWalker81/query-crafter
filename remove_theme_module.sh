#!/bin/bash

echo "Removing theme module from main crate to fix incremental builds..."
echo "=========================================================="

# Step 1: Remove theme module from main crate
echo "1. Removing theme module from src/lib.rs and src/main.rs..."
sed -i '/pub mod theme;/d' src/lib.rs 2>/dev/null || true
sed -i '/mod theme;/d' src/main.rs 2>/dev/null || true

# Step 2: Update all files to use query_crafter_theme directly
echo "2. Updating all imports to use query_crafter_theme..."

# Files that need updating
FILES=(
    "src/components/db.rs"
    "src/components/vim.rs"
    "src/autocomplete_widget.rs"
    "src/editor_common.rs"
)

for file in "${FILES[@]}"; do
    if [ -f "$file" ]; then
        echo "   Updating $file..."
        # Replace crate::theme::Theme:: with query_crafter_theme::
        sed -i 's/crate::theme::Theme::/query_crafter_theme::/g' "$file"
        # Replace use crate::theme with use query_crafter_theme
        sed -i 's/use crate::theme/use query_crafter_theme/g' "$file"
        # Replace Theme:: with query_crafter_theme:: (for standalone Theme:: references)
        sed -i 's/\bTheme::/query_crafter_theme::/g' "$file"
    fi
done

# Step 3: Remove the theme.rs file
echo "3. Removing src/theme.rs..."
rm -f src/theme.rs

# Step 4: Check for any remaining references
echo -e "\n4. Checking for remaining theme references..."
if grep -r "crate::theme" src/ --include="*.rs" 2>/dev/null; then
    echo "WARNING: Found remaining crate::theme references!"
else
    echo "✓ No remaining crate::theme references found"
fi

echo -e "\nTheme module removal complete!"