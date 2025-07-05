# SelectionMode Navigation Fix

## Issues Found:

1. **Row Mode Navigation**:
   - `detail_row_index` is incorrectly navigating through columns instead of displaying row details
   - Up/Down keys should navigate through different rows, not columns within a row

2. **Cell Mode Navigation**:
   - Horizontal navigation (h/l keys) is handled in handlers.rs but not properly updating state
   - The Action::Render being returned might not be a valid action

3. **Scrolling**:
   - Horizontal scrolling is disabled in Cell mode but should work differently

## Required Changes:

### 1. Fix Row Mode Navigation (state.rs lines 131-137, 177-183):
The Row mode should show details of the selected row, and up/down should navigate between rows, not within columns of a single row.

### 2. Fix Cell Mode Horizontal Navigation:
The state.rs file needs to handle horizontal cell movement properly.

### 3. Fix Action::Render:
This action doesn't seem to exist in the Action enum and needs to be removed or replaced.

## Implementation Plan:

1. Update state.rs to fix Row mode navigation
2. Ensure Cell mode horizontal navigation updates state properly
3. Remove or replace Action::Render with proper state updates
4. Test all selection modes thoroughly