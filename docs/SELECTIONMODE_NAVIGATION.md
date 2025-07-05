# SelectionMode Navigation Documentation

## Overview
Query Crafter's results view supports multiple selection modes for navigating and interacting with query results.

## Selection Modes

### 1. Table Mode (Default)
- **Description**: Standard table navigation mode
- **Navigation**:
  - `j`/`k` or `↓`/`↑`: Navigate rows up/down
  - `h`/`l` or `←`/`→`: Scroll columns left/right (scrolls entire view)
  - `PgUp`/`PgDn`: Navigate by pages
  - `Home`/`End`: Jump to first/last row
- **Actions**:
  - `y`: Copy current row (tab-separated)
  - `/`: Search within results
  - `e`: Export results to CSV

### 2. Row Mode
- **Activation**: Press `Space` to toggle
- **Description**: Shows detailed view of the selected row
- **Navigation**:
  - `j`/`k` or `↓`/`↑`: Navigate between different rows (not within columns)
  - The detailed view shows all columns for the selected row
- **Exit**: Press `Space` or `Esc` to return to Table mode

### 3. Cell Mode
- **Activation**: Press `v` to enter cell selection mode
- **Description**: Navigate and select individual cells
- **Navigation**:
  - `j`/`k` or `↓`/`↑`: Move between rows
  - `h`/`l` or `←`/`→`: Move between cells in the current row
  - Auto-scrolls horizontally to keep selected cell visible
- **Actions**:
  - `y` or `c`: Copy selected cell value
- **Exit**: Press `Esc` to return to Table mode

### 4. Preview Mode
- **Activation**: Press `p` to toggle
- **Description**: Shows a popup preview of the current row
- **Exit**: Press `p` again or `Esc`

## Key Bindings Summary

| Key | Table Mode | Row Mode | Cell Mode |
|-----|------------|----------|-----------|
| `j`/`↓` | Next row | Next row | Next row |
| `k`/`↑` | Previous row | Previous row | Previous row |
| `h`/`←` | Scroll left | - | Previous cell |
| `l`/`→` | Scroll right | - | Next cell |
| `Space` | → Row mode | → Table mode | - |
| `v` | → Cell mode | → Cell mode | - |
| `p` | → Preview | → Preview | → Preview |
| `y` | Copy row | Copy row | Copy cell |
| `Esc` | - | → Table mode | → Table mode |

## Implementation Details

### State Management
- `selection_mode`: Enum tracking current mode (Table, Row, Cell, Preview)
- `selected_row_index`: Current row position
- `selected_cell_index`: Current cell position (used in Cell mode)
- `detail_row_index`: Reserved for future row detail navigation
- `horizonal_scroll_offset`: Column scroll position (pages of columns)

### Scrolling Behavior
- **Table Mode**: Scrolls entire view by column pages (3 columns at a time)
- **Cell Mode**: Auto-scrolls to keep selected cell visible
- **Row/Preview Mode**: No horizontal scrolling

### Search Integration
- Search works in all modes
- Filtered results maintain proper navigation
- Cell selection respects filtered results