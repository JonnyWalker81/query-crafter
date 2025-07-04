# Query Crafter Test Suite

This test suite provides comprehensive testing for the Query Crafter TUI application using multiple testing strategies.

## Test Structure

```
tests/
├── test_utils/         # Shared testing utilities
│   ├── terminal.rs     # Mock terminal and event driver
│   ├── component.rs    # Component test harness
│   ├── builders.rs     # Event and action builders
│   ├── fixtures.rs     # Test data fixtures
│   └── assertions.rs   # Common test assertions
├── unit/              # Unit tests
│   ├── actions.rs      # Action enum tests
│   └── components/     # Component-specific tests
│       ├── vim_test.rs # Vim editor tests
│       └── db_test.rs  # Database component tests
├── integration/       # Integration tests
│   ├── event_flow.rs   # Event to action flow tests
│   ├── vim_mode.rs     # Vim operation integration
│   ├── navigation.rs   # Navigation behavior tests
│   └── database.rs     # Database integration tests
├── visual/            # Visual regression tests
│   ├── snapshots.rs    # Full UI snapshot tests
│   └── components.rs   # Component rendering tests
└── e2e/              # End-to-end tests
    └── workflows.rs    # Complete user workflows
```

## Running Tests

### Run all tests
```bash
cargo test
```

### Run specific test categories
```bash
cargo test --test unit
cargo test --test integration
cargo test --test visual
cargo test --test e2e
```

### Run tests with output
```bash
cargo test -- --nocapture
```

### Run a specific test
```bash
cargo test test_vim_format_operator
```

## Test Categories

### Unit Tests
Fast, isolated tests for individual components and functions:
- Action creation and routing
- Component state management
- Vim editor operations
- SQL formatting

### Integration Tests
Tests that verify component interactions:
- Keyboard event → action flow
- Multi-key sequences (gg, ==)
- Component focus switching
- Database query execution

### Visual Regression Tests
Snapshot tests using `insta` to detect UI changes:
- Initial layout rendering
- Component states (focused, populated)
- Popup overlays (help, errors)
- Results table rendering

To review snapshot changes:
```bash
cargo insta review
```

### End-to-End Tests
Complete user workflow tests:
- Query workflow (browse → edit → execute → view)
- Search and filter operations
- Vim editing workflows
- Error recovery scenarios

## Key Testing Utilities

### MockTerminal
Simulates terminal events without requiring a real terminal:
```rust
let (driver, rx) = TestDriver::new();
let terminal = MockTerminal::new(rx);

// Send events
driver.send_key('j').await.unwrap();
driver.send_ctrl('s').await.unwrap();
```

### ComponentTestHarness
Wraps components for testing with a TestBackend:
```rust
let harness = ComponentTestHarness::new(component)?;
harness.send_event(event)?;
let buffer = harness.get_buffer_content();
```

### EventBuilder
Fluent API for creating event sequences:
```rust
let events = EventBuilder::new()
    .keys("SELECT * FROM users")
    .ctrl('enter')
    .build();
```

### Assertions
Common assertions for TUI testing:
```rust
buffer.assert_contains("Query Results");
buffer.assert_popup_visible();
assert_vim_mode(&buffer, "Normal");
```

## Adding New Tests

1. **Unit Test**: Add to appropriate file in `tests/unit/`
2. **Integration Test**: Add to relevant file in `tests/integration/`
3. **Visual Test**: Add snapshot test to `tests/visual/snapshots.rs`
4. **E2E Test**: Add workflow to `tests/e2e/workflows.rs`

## Test Best Practices

1. **Consistent Terminal Size**: Use 80x24 for visual tests
2. **Deterministic State**: Mock time-based operations
3. **Test Isolation**: Each test gets fresh component state
4. **Descriptive Names**: Use clear test function names
5. **Snapshot Descriptions**: Add context to snapshot tests

## Debugging Failed Tests

1. **Visual Tests**: Use `cargo insta review` to see diffs
2. **Integration Tests**: Add `--nocapture` to see debug output
3. **Check Test Logs**: Look for assertion messages
4. **Isolate Test**: Run single test with full output

## CI/CD Integration

The test suite is designed to run in CI environments:
- No terminal required (uses TestBackend)
- Deterministic output (no timing dependencies)
- Snapshot tests can be updated via CI

## Known Limitations

1. **Color Assertions**: TestBackend doesn't capture colors
2. **Async Testing**: Some operations require sleep delays
3. **Real Database**: Integration tests use SQLite in-memory

## Contributing

When adding new features:
1. Add unit tests for new actions/components
2. Add integration tests for interactions
3. Add snapshot tests for UI changes
4. Add e2e tests for user workflows