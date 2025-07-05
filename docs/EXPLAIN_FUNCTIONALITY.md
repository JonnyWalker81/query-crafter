# EXPLAIN Functionality in Query Crafter

Query Crafter now supports enhanced EXPLAIN functionality for analyzing query execution plans in both PostgreSQL and SQLite databases.

## Features

### 1. Quick EXPLAIN Shortcuts
- **Alt+E**: Prefix current query with `EXPLAIN`
- **Alt+X**: Prefix current query with `EXPLAIN ANALYZE`
- If the query already has EXPLAIN, these shortcuts are smart enough not to duplicate it

### 2. EXPLAIN View Toggle
The 'x' key in Results view toggles between EXPLAIN and regular query execution:
- **On regular query results**: Press 'x' to re-run the query with EXPLAIN prefix to see the execution plan
- **On EXPLAIN results**: Press 'x' to remove EXPLAIN and run the actual query to see table data
- This allows you to quickly switch between viewing data and analyzing query performance

### 3. Enhanced EXPLAIN Visualization

#### PostgreSQL EXPLAIN Output
- Single-column query plans are rendered as formatted text
- Color-coded operations:
  - **Yellow**: Sequential scans (potential performance issue)
  - **Green**: Index scans (good performance)
  - **Blue**: Join operations
  - **Gray**: Sort operations
- Scrollable view with line numbers

#### SQLite EXPLAIN QUERY PLAN
- Multi-column output rendered as a table
- Performance metrics highlighted:
  - **Red**: Operations taking >1000ms
  - **Yellow**: Operations taking >100ms
- Standard table navigation applies

### 4. Usage Examples

#### PostgreSQL
```sql
-- Basic EXPLAIN
EXPLAIN SELECT * FROM users WHERE id = 1;

-- EXPLAIN with execution statistics
EXPLAIN ANALYZE SELECT * FROM users WHERE id = 1;

-- EXPLAIN with all options
EXPLAIN (ANALYZE, BUFFERS, VERBOSE) SELECT * FROM users WHERE id = 1;
```

#### SQLite
```sql
-- Query plan
EXPLAIN QUERY PLAN SELECT * FROM users WHERE id = 1;

-- Detailed EXPLAIN
EXPLAIN SELECT * FROM users WHERE id = 1;
```

### 5. Keyboard Shortcuts Summary

| Key | Action | Context |
|-----|--------|---------|
| Alt+E | Add EXPLAIN to query | Query editor |
| Alt+X | Add EXPLAIN ANALYZE to query | Query editor |
| x | Toggle EXPLAIN view | Results view |
| j/k | Navigate rows | Results view |
| h/l | Scroll horizontally | Results view (table mode) |

## Implementation Details

### Code Structure
- **Actions**: Added `ExplainQuery`, `ExplainAnalyzeQuery`, and `ToggleExplainView` actions
- **State Management**: Added `is_explain_view` and `is_explain_query` flags to track state
- **Rendering**: New functions `render_explain_output`, `render_explain_text_output`, and `render_explain_table_output`
- **Auto-detection**: Queries starting with "EXPLAIN" automatically enable EXPLAIN view

### Database-Specific Handling
The system detects the type of EXPLAIN output:
- Single column with "query plan" header → PostgreSQL text format
- Multiple columns → SQLite or PostgreSQL table format

### Future Enhancements
The following features are planned for future releases:
1. Tree-style indentation for nested plan nodes
2. Collapsible/expandable plan nodes
3. Export EXPLAIN output to external visualizers
4. Vertical/horizontal toggle for wide EXPLAIN outputs
5. Search within EXPLAIN output
6. Cost-based highlighting and analysis