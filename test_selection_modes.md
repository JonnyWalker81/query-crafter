# Selection Mode Navigation Test Plan

## Overview
This document describes how to test the fixed selection mode navigation in Query Crafter.

## Test Steps

### 1. Table Mode (Default)
- Run a query that returns multiple rows
- Use j/k or arrow keys to navigate rows
- Use h/l to scroll columns horizontally
- Press space to enter Row mode

### 2. Row Mode
- From Table mode, press space
- Use j/k to navigate between different rows (not columns within a row)
- Press space again to return to Table mode
- Press ESC to return to Table mode

### 3. Cell Mode  
- From Table mode, press 'v' to enter Cell mode
- Use h/l or arrow keys to navigate individual cells
- Use j/k to move between rows while maintaining cell position
- The view should auto-scroll to keep selected cell visible
- Press ESC to return to Table mode

### 4. Preview Mode
- From Table mode, press 'p' to enter Preview mode
- Navigate rows to see cell details
- Press 'p' again or ESC to return to Table mode

### 5. Search Mode
- In any mode, press '/' to enter search
- Type search query
- Use j/k to navigate filtered results
- Press Enter or ESC to exit search

## Key Bindings Summary

### Results Navigation
- **j/k or ↑/↓**: Navigate rows
- **h/l or ←/→**: Navigate columns (Table mode) or cells (Cell mode)
- **Space**: Toggle Row mode
- **v**: Enter Cell mode
- **p**: Toggle Preview mode
- **/**: Search results
- **y**: Copy current cell/row
- **ESC**: Exit mode/search

## Fixed Issues
1. Row mode now correctly navigates between rows (not columns)
2. Cell mode navigation with h/l keys works properly
3. Auto-scrolling keeps selected cell visible
4. Search results navigation maintains proper indices