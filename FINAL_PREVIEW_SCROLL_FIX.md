# Preview Mode Scrolling and Column Count Display Fix

## Summary
Fixed preview mode scrolling to properly display all row data and added column count information to both the results table header and preview popup.

## Changes Made

### 1. Preview Mode Scrolling Fix
- Added proper scroll boundary checking to prevent scrolling beyond content
- Calculates total lines and visible lines for accurate scrolling
- Shows scroll percentage indicator in top-right corner
- Ensures all columns can be viewed by scrolling with j/k keys

### 2. Column Count Display
Added column count information to:

#### Results Table Header
- Shows row and column counts in the title bar
- Format: `[3] Results (100 rows, 15 cols)`
- When filtering: `[3] Results (25/100 rows, 15 cols)`

#### Preview Popup Header  
- Shows both row position and total columns
- Format: `Row 5 of 100  |  Columns: 15`

### 3. Enhanced Preview Display
The preview now shows:
```
Row 5 of 100  |  Columns: 15
────────────────────────────────────────

id:
5

name:
John Doe

email:
john.doe@example.com

[... all columns continue ...]
```

### 4. Scroll Indicator
- Shows percentage in top-right when content exceeds viewport
- Format: ` 25% `
- Updates as you scroll through content

## Navigation
### Preview Mode (p key)
- **j/k or ↑/↓**: Scroll line by line
- **PageUp/PageDown**: Scroll by 10 lines
- **Home**: Jump to top
- **y**: Copy entire row as TSV
- **c**: Copy current cell value
- **Esc or p**: Exit preview

### Row Mode (Space key)
- Split view with detail pane
- **j/k**: Navigate through columns in detail pane
- **Space or Esc**: Return to table mode

## Testing
1. Run a query with many columns (15+)
2. Press 'p' to enter preview mode
3. Verify column count shows in header
4. Scroll with j/k to see all columns
5. Check scroll percentage indicator updates
6. Verify all data is accessible via scrolling