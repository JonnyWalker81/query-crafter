# Preview Table Mode Implementation

## Summary
Reimplemented the preview popup to show all columns in a horizontal table format with scrolling and JSON copy support.

## Changes Made

### 1. Table-Based Preview Display
- Changed from vertical list (column: value) to horizontal table view
- Shows all columns with headers in the first row and values in the second row
- Popup size: 90% width, 60% height (centered)

### 2. Navigation Controls
- **h/l or ←/→**: Scroll columns horizontally
- **Home**: Jump to first column
- **End**: Jump to last column  
- **Esc or p**: Exit preview mode

### 3. Copy Functions
- **y**: Copy the currently visible/highlighted cell value (first visible column)
- **Y**: Copy entire row as formatted JSON object

### 4. Visual Indicators
- Shows current column range (e.g., "Cols 5-10 of 30")
- Highlights the first visible column for 'y' copy functionality
- Clear title with all keyboard shortcuts

## Implementation Details

### Preview Popup Rendering
```rust
// Create a table showing all columns
let skip_count = self.preview_scroll_offset as usize;
let visible_cols = (inner.width / 20).max(1) as usize; // Estimate columns that fit

// Header row with column names
let header_cells: Vec<_> = self.selected_headers
    .iter()
    .skip(skip_count)
    .take(visible_cols)
    .map(|h| Cell::from(h.as_str()).style(theme::header()))
    .collect();

// Data row with values
let data_cells: Vec<_> = row
    .iter()
    .skip(skip_count)
    .take(visible_cols)
    .enumerate()
    .map(|(idx, value)| {
        let content = if value.is_empty() {
            "(empty)"
        } else if value == "NULL" {
            "NULL"  
        } else {
            value.as_str()
        };
        // Highlight first column for 'y' copy
        if idx == 0 {
            Cell::from(content).style(theme::selection_active())
        } else {
            Cell::from(content)
        }
    })
    .collect();
```

### JSON Copy Implementation
```rust
fn copy_row_as_json(&mut self) {
    if let Some(row) = self.get_current_row() {
        use std::collections::HashMap;
        
        // Create JSON object with column names as keys
        let mut json_map = HashMap::new();
        for (header, value) in self.selected_headers.iter().zip(row.iter()) {
            json_map.insert(header.as_str(), value.as_str());
        }
        
        // Convert to pretty-printed JSON
        if let Ok(json_string) = serde_json::to_string_pretty(&json_map) {
            if let Ok(mut ctx) = clipboard::ClipboardContext::new() {
                let _ = ctx.set_contents(json_string).ok();
            }
        }
    }
}
```

## Example Output

### Table View in Preview
```
┌─ Row 5 of 100 - [h/l] Scroll Columns [y] Copy Cell [Y] Copy JSON [<Esc>] Close ─┐
│ id    │ name      │ email                │ department  │ salary    │           │
│ 5     │ John Doe  │ john.doe@example.com │ Engineering │ 75000.00  │           │
│                                                             Cols 1-5 of 15      │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### JSON Copy Output (Y key)
```json
{
  "id": "5",
  "name": "John Doe",
  "email": "john.doe@example.com",
  "department": "Engineering",
  "salary": "75000.00",
  "hire_date": "2020-03-15",
  "status": "Active"
}
```

## Testing
1. Run a query with multiple columns
2. Press 'p' to open preview popup
3. Use h/l to scroll through columns
4. Press 'y' to copy visible cell value
5. Press 'Y' to copy entire row as JSON
6. Verify JSON format in clipboard