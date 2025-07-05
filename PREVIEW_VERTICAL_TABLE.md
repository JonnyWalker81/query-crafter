# Preview Mode - Vertical Table Implementation

## Summary
Implemented the row preview popup as a vertical table with two columns: Column names on the left, values on the right. This provides a clear, scrollable view of all fields in the selected row.

## Visual Design

### Layout
```
┌─ Row 5 of 100 - [j/k] Scroll [y] Copy Cell [Y] Copy JSON [<Esc>] Close ─┐
│ Column              │ Value                                              │
│─────────────────────┼────────────────────────────────────────────────────│
│ id                  │ 5                                                  │
│ name                │ John Doe                                           │
│ email               │ john.doe@example.com                               │
│ department          │ Engineering                                        │
│ salary              │ 75000.00                                           │
│ hire_date           │ 2020-03-15                                         │
│ status              │ Active                                             │
│ notes               │ Long text field with multiple lines...             │
│                     │                                     5 of 15        │
└───────────────────────────────────────────────────────────────────────────┘
```

## Features

### 1. Vertical Table Format
- Two-column layout: Column names (30%) | Values (70%)
- Header row with "Column" and "Value" labels
- Clean separation between field names and data

### 2. Navigation
- **j/k or ↑/↓**: Scroll down/up one row
- **PageDown/PageUp**: Scroll 10 rows at a time
- **Home**: Jump to first field
- **End**: Jump to last field
- **Esc or p**: Close preview

### 3. Copy Functions
- **y**: Copy the value of the currently highlighted field (first visible row)
- **Y**: Copy entire row as formatted JSON object

### 4. Visual Indicators
- Highlighted row shows which field 'y' will copy
- Scroll indicator shows position (e.g., "5 of 15")
- Popup size: 70% width, 80% height for comfortable viewing

## Implementation Details

### Vertical Table Structure
```rust
// Build table rows for each column/value pair
let table_rows: Vec<ratatui::widgets::Row> = self.selected_headers
    .iter()
    .zip(row.iter())
    .skip(skip_count)
    .take(visible_rows)
    .enumerate()
    .map(|(idx, (header, value))| {
        let cells = vec![
            Cell::from(header.as_str()),
            Cell::from(if value.is_empty() {
                "(empty)"
            } else if value == "NULL" {
                "NULL"
            } else {
                value.as_str()
            })
        ];
        
        // Highlight the first visible row
        if idx == 0 {
            ratatui::widgets::Row::new(cells).style(theme::selection_active())
        } else {
            ratatui::widgets::Row::new(cells)
        }
    })
    .collect();
```

### JSON Export
When pressing 'Y', the row is exported as pretty-printed JSON:
```json
{
  "id": "5",
  "name": "John Doe",
  "email": "john.doe@example.com",
  "department": "Engineering",
  "salary": "75000.00",
  "hire_date": "2020-03-15",
  "status": "Active",
  "notes": "Long text field..."
}
```

## Benefits
1. **Clear Layout**: Column names and values are clearly separated
2. **Efficient Space Usage**: Vertical layout works well for rows with many columns
3. **Easy Navigation**: Natural up/down scrolling through fields
4. **Flexible Copying**: Copy individual values or entire row as JSON