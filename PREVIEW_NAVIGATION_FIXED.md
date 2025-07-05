# Preview Navigation Fix

## Summary
Fixed the preview popup navigation to properly move the selection highlight when using j/k keys, with automatic scrolling to keep the selected row visible.

## Changes Made

### 1. Added Selection Tracking
- New field `preview_selected_index` tracks which row is currently selected
- Separate from `preview_scroll_offset` which tracks the viewport position

### 2. Updated Navigation Logic
- **j/k**: Move selection down/up by one row
  - Automatically scrolls viewport if selection moves out of view
  - Selection highlight follows the cursor
- **PageUp/PageDown**: Jump selection by 10 rows
  - Updates both selection and scroll position
- **Home/End**: Jump to first/last row
  - Properly sets both selection and scroll

### 3. Visual Feedback
- Selected row is highlighted with `theme::selection_active()`
- Scroll indicator shows selected row position (e.g., "15 of 30")
- Highlight moves with navigation, not just the scroll position

### 4. Copy Behavior
- 'y' now copies the value from the selected row (not just first visible)
- 'Y' still copies entire row as JSON

## Implementation Details

```rust
// Navigation with selection tracking
KeyCode::Down | KeyCode::Char('j') => {
    if self.preview_selected_index < self.selected_headers.len().saturating_sub(1) {
        self.preview_selected_index += 1;
        
        // Auto-scroll to keep selection visible
        let visible_rows = 20; // Estimate based on popup height
        if self.preview_selected_index >= self.preview_scroll_offset as usize + visible_rows {
            self.preview_scroll_offset = (self.preview_selected_index + 1).saturating_sub(visible_rows) as u16;
        }
    }
}

// Rendering with selection highlight
if skip_count + idx == self.preview_selected_index {
    ratatui::widgets::Row::new(cells).style(theme::selection_active())
} else {
    ratatui::widgets::Row::new(cells)
}
```

## User Experience
1. Press 'p' to open preview
2. Use j/k to navigate - selection highlight moves
3. Table automatically scrolls when selection reaches edge
4. Press 'y' to copy the currently selected value
5. Indicator shows "15 of 30" for current position

The preview popup now provides proper interactive navigation with visual feedback, making it easy to browse through all fields and copy specific values.