pub mod terminal;
pub mod component;
pub mod builders;
pub mod fixtures;
pub mod assertions;

// Re-export commonly used items
pub use terminal::TestDriver;
pub use component::{ComponentTestHarness, TestEnvironment};
pub use builders::EventBuilder;

use std::time::Duration;

pub const TEST_TERMINAL_WIDTH: u16 = 80;
pub const TEST_TERMINAL_HEIGHT: u16 = 24;
pub const TEST_TICK_RATE: Duration = Duration::from_millis(10);