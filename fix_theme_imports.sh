#!/bin/bash

echo "Fixing theme imports to use external crate directly..."

# Replace all crate::theme::Theme:: with query_crafter_theme::
echo "Updating db.rs..."
sed -i 's/crate::theme::Theme::/query_crafter_theme::/g' src/components/db.rs

# Also update other files
echo "Updating vim.rs..."
sed -i 's/crate::theme::Theme::/query_crafter_theme::/g' src/components/vim.rs

echo "Updating autocomplete_widget.rs..."
sed -i 's/crate::theme::Theme::/query_crafter_theme::/g' src/autocomplete_widget.rs

echo "Updating editor_common.rs..."
sed -i 's/crate::theme::Theme::/query_crafter_theme::/g' src/editor_common.rs

# Check the changes
echo -e "\nChanges made:"
echo "db.rs: $(grep -c "query_crafter_theme::" src/components/db.rs) references"
echo "vim.rs: $(grep -c "query_crafter_theme::" src/components/vim.rs) references"
echo "autocomplete_widget.rs: $(grep -c "query_crafter_theme::" src/autocomplete_widget.rs) references"
echo "editor_common.rs: $(grep -c "query_crafter_theme::" src/editor_common.rs) references"