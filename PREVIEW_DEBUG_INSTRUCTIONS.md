# Preview Navigation Debug Instructions

## What We've Done

Added debug logging to trace the preview popup navigation issue. The debug statements will help identify:

1. Whether preview mode is being entered correctly
2. Whether the j/k key events are reaching the preview handler
3. What the current state is when keys are pressed

## Debug Output Locations

### In `src/components/db/handlers.rs`:

1. **Lines 47-48**: Shows when in preview mode and what key was pressed
2. **Lines 51, 55**: Shows when k/Up is pressed and index updates  
3. **Lines 64, 68**: Shows when j/Down is pressed and index updates
4. **Lines 577, 583-584, 592-593**: Shows when entering/exiting preview mode via Space/Enter
5. **Lines 603, 609-610, 618-619**: Shows when entering/exiting preview mode via 'p' key

## How to Test

1. Build and run the application:
   ```bash
   cargo run
   ```

2. Execute a query that returns results

3. Navigate to the Results tab (press '3')

4. Press 'p' or Space to open the preview popup

5. Try pressing 'j' and 'k' to navigate

6. Check the terminal output for debug messages

## What to Look For

The debug output should show:

1. "DEBUG: Entering preview mode" when you press 'p' or Space
2. "DEBUG: In preview mode, key: Char('j')" when you press 'j'
3. "DEBUG: j/Down pressed in preview mode"
4. "DEBUG: Updated preview_selected_index to X"

If you see these messages but navigation still doesn't work, the issue is in the rendering.
If you don't see these messages, the issue is with event handling or state management.

## To Remove Debug Output

Once the issue is identified and fixed, remove all lines containing `eprintln!("DEBUG:` from:
- `src/components/db/handlers.rs`