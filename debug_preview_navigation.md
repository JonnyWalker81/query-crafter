# Debug Preview Navigation Issue

## Problem Analysis

The preview popup navigation (j/k keys) is not working. Based on code analysis:

### Key Findings:

1. **Event Handling Order (handlers.rs:45-125)**:
   - Preview mode is handled BEFORE component-specific handling ✓
   - When in preview mode and j/k pressed, it correctly:
     - Updates `preview_selected_index`
     - Updates `preview_scroll_offset` 
     - Returns `Ok(Some(Action::Render))` to trigger redraw

2. **Safety Checks (handlers.rs:568,589)**:
   - Code correctly checks `!selected_headers.is_empty()` before entering preview mode ✓

3. **Rendering (rendering.rs:570-664)**:
   - Preview popup renders correctly
   - Uses `preview_selected_index` to highlight selected row
   - Shows correct scroll indicator

4. **Action Processing (app.rs)**:
   - `Action::Render` is properly handled and triggers a redraw ✓

## Potential Issues to Check:

1. **Debug Logging**: Add debug prints to verify:
   ```rust
   // In handlers.rs, line 46
   if self.selection_mode == SelectionMode::Preview {
       eprintln!("DEBUG: In preview mode, key: {:?}", key.code);
       match key.code {
           KeyCode::Down | KeyCode::Char('j') => {
               eprintln!("DEBUG: j pressed, current index: {}, headers len: {}", 
                        self.preview_selected_index, self.selected_headers.len());
   ```

2. **Check if preview mode is actually active**:
   - The popup might be showing but `selection_mode` might not be `SelectionMode::Preview`

3. **Verify selected_headers is populated**:
   - Even though we check for empty, it might be cleared somewhere

## Recommended Debug Steps:

1. Add debug logging to trace the exact flow
2. Check if another component or handler is consuming the j/k keys
3. Verify the selection_mode state when the popup is shown
4. Check if there's a race condition with state updates

## Code to Add for Debugging:

In `src/components/db/handlers.rs`, add after line 45:
```rust
eprintln!("DEBUG: selection_mode = {:?}, preview_index = {}, headers_len = {}", 
         self.selection_mode, self.preview_selected_index, self.selected_headers.len());
```

In the j/k handlers (lines 48-69):
```rust
eprintln!("DEBUG: Before update - preview_selected_index = {}", self.preview_selected_index);
// ... existing code ...
eprintln!("DEBUG: After update - preview_selected_index = {}", self.preview_selected_index);
```

This will help identify if:
- The preview mode handler is being reached
- The state is being updated correctly
- The render action is being sent