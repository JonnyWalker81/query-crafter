# Row Details and Preview Mode Fixes

## Summary
Fixed the Row mode navigation and enhanced the Preview popup to display all data with scrolling support.

## Changes Made

### 1. Row Mode Navigation Fix
**Problem**: Row mode was navigating between different rows instead of navigating through columns in the detail pane.
**Fix**: Restored original behavior where j/k navigates through columns in the detail pane using `detail_row_index`.

```rust
// Now correctly navigates through columns in detail pane
SelectionMode::Row => {
    // In Row mode, navigate through columns in the detail pane
    if !self.query_results.is_empty() && self.selected_row_index < self.query_results.len() {
        let num_columns = self.query_results[self.selected_row_index].len();
        if self.detail_row_index < num_columns - 1 {
            self.detail_row_index += 1;
        } else {
            self.detail_row_index = 0; // Wrap to top
        }
    }
}
```

### 2. Preview Popup Enhancements
- Increased popup size to 90% width and 80% height for more content
- Added scrolling support with j/k, PageUp/PageDown, and Home keys
- Enhanced header to show row and column position
- Added horizontal separator between header and content

### 3. Preview Mode Navigation
Added complete navigation support for Preview mode:
- **j/k or ↑/↓**: Scroll content up/down by one line
- **PageUp/PageDown**: Scroll by 10 lines
- **Home**: Jump to top of content
- **Esc or p**: Exit preview mode
- **c or y**: Copy current cell value

## How It Works

### Row Mode (Space key)
- Shows a split view with table on left (60%) and detail pane on right (40%)
- Detail pane displays all columns for the selected row
- j/k navigates through columns in the detail pane (highlighted selection)
- Space or Esc returns to Table mode

### Preview Mode (p key)
- Shows a large popup overlay with current cell details
- Displays row/column position and full cell content
- Content can be scrolled if it exceeds popup height
- Supports text wrapping for long content
- Maintains scroll position while in preview mode

## Testing
1. Run a query that returns data with long text fields
2. Press Space to enter Row mode and use j/k to navigate columns
3. Press p to enter Preview mode and test scrolling with j/k
4. Verify that all data is visible and scrollable in both modes