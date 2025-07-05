# Preview Mode Fix - Complete Row Display

## Summary
Fixed the Preview mode to display all columns and values for the entire row, not just a single cell.

## Changes Made

### 1. Preview Content Display
**Before**: Only showed the value of the currently selected cell
**After**: Shows all columns and values for the entire selected row

The preview now displays:
- Row number (e.g., "Row 5 of 100")
- A separator line
- All column names and their values in a vertical list
- Special formatting for empty values and NULLs
- Multi-line values are properly indented

### 2. Enhanced Formatting
```
Row 5 of 100
────────────────────────────────────────

id:
5

name:
John Doe

email:
john.doe@example.com

description:
This is a multi-line
  description that continues
  on multiple lines

status:
(empty)

created_at:
2024-01-15 10:30:00
```

### 3. Navigation and Actions
- **j/k or ↑/↓**: Scroll through the preview content
- **PageUp/PageDown**: Scroll by 10 lines
- **Home**: Jump to top
- **y**: Copy entire row as tab-separated values
- **c**: Copy current cell value (based on selected_cell_index)
- **Esc or p**: Close preview and return to table view

## How It Works

The Preview mode now:
1. Collects all column headers and values for the selected row
2. Formats them in a readable vertical layout
3. Handles special cases (empty values, NULLs, multi-line text)
4. Provides scrolling for rows with many columns or long values
5. Maintains the ability to copy either the full row or individual cells

## Testing
1. Run a query that returns multiple columns
2. Navigate to a row with varied data (nulls, empty values, long text)
3. Press 'p' to enter Preview mode
4. Verify all columns and values are displayed
5. Test scrolling with j/k for rows with many columns
6. Test copying with 'y' (full row) and 'c' (current cell)