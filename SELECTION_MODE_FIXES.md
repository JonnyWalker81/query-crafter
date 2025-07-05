# Selection Mode Navigation Fixes

## Summary
Fixed the selection mode navigation issues in the query results table to match the original behavior.

## Changes Made

### 1. Row Mode Navigation Fix
**Problem**: Row mode was incorrectly using `detail_row_index` to navigate through columns within a single row.
**Fix**: Changed Row mode to navigate between different rows (same as Table mode).

```rust
// Before: Navigate through columns in a single row
if self.detail_row_index < self.query_results[self.selected_row_index].len() - 1 {
    self.detail_row_index += 1;
}

// After: Navigate between rows
if self.selected_row_index < self.query_results.len() - 1 {
    self.selected_row_index += 1;
}
```

### 2. Cell Mode Navigation
**Status**: Already implemented correctly with h/l keys for cell navigation.
- Moves cell selection left/right
- Auto-scrolls to keep selected cell visible
- j/k moves between rows while maintaining cell position

### 3. Code Cleanup
- Removed duplicate `filter_results_simple()` method
- Updated all references to use the single `filter_results()` method
- Added proper index management for filtered results

## Selection Modes Overview

### Table Mode (Default)
- Full row selection
- j/k: Navigate rows
- h/l: Scroll columns horizontally
- Space: Enter Row mode
- v: Enter Cell mode
- p: Enter Preview mode

### Row Mode
- Shows detailed view of selected row
- j/k: Navigate between different rows
- Space or ESC: Return to Table mode

### Cell Mode
- Individual cell selection
- h/l: Navigate cells horizontally
- j/k: Navigate rows vertically
- Auto-scrolls to keep cell visible
- ESC: Return to Table mode

### Preview Mode
- Shows cell details in a popup
- j/k: Navigate rows
- p or ESC: Return to Table mode

## Testing
To test the fixes:
1. Run a query that returns multiple rows and columns
2. Test each selection mode using the key bindings above
3. Verify navigation works as expected in each mode
4. Test search functionality with '/' key