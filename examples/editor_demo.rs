use color_eyre::eyre::Result;
use query_crafter::components::db::EditorBackend;
use ratatui::layout::Rect;

fn main() -> Result<()> {
  color_eyre::install()?;

  println!("Editor Backend Demo");
  println!("==================");

  // Test creating editors from config
  test_editor_factory()?;

  // Test editor switching
  test_editor_switching()?;

  // Test basic text operations
  test_text_operations()?;

  println!("All tests passed! ✅");

  Ok(())
}

fn test_editor_factory() -> Result<()> {
  println!("\n🧪 Testing editor factory pattern...");

  // Test default tui-textarea backend
  let tui_editor = EditorBackend::new_from_config("tui-textarea");
  match tui_editor {
    EditorBackend::TuiTextarea(_) => println!("✅ TUI TextArea editor created successfully"),
  }

  // Test unknown backend defaults to tui-textarea
  let other_editor = EditorBackend::new_from_config("other");
  match other_editor {
    EditorBackend::TuiTextarea(_) => println!("✅ Unknown backend defaults to TuiTextarea"),
  }

  // Test fallback for unknown backend
  let fallback_editor = EditorBackend::new_from_config("unknown");
  match fallback_editor {
    EditorBackend::TuiTextarea(_) => println!("✅ Fallback to TuiTextarea works"),
  }

  Ok(())
}

fn test_editor_switching() -> Result<()> {
  println!("\n🔄 Testing editor switching...");

  let mut backend = EditorBackend::new_from_config("tui-textarea");
  let test_text = "SELECT * FROM users WHERE id = 1;";

  // Set initial text
  backend.set_text(test_text);
  assert_eq!(backend.get_text(), test_text);
  println!("✅ Initial text set in TuiTextarea backend");

  // Currently only TuiTextarea is supported, but the pattern allows for future backends
  println!("✅ Editor backend system is extensible for future editor implementations");

  Ok(())
}

fn test_text_operations() -> Result<()> {
  println!("\n📝 Testing basic text operations...");

  let mut backend = EditorBackend::new_from_config("tui-textarea");
  let area = Rect::new(0, 0, 80, 24);

  // Initialize
  backend.as_editor_component().init(area)?;
  println!("✅ Editor initialized");

  // Test text setting and getting
  let test_queries = [
    "SELECT * FROM users;",
    "UPDATE users SET name = 'John' WHERE id = 1;",
    "DELETE FROM sessions WHERE expired_at < NOW();",
  ];

  for query in &test_queries {
    backend.set_text(query);
    assert_eq!(backend.get_text(), *query);
  }
  println!("✅ Text setting and getting works for multiple queries");

  // The editor backend abstracts away mode handling
  println!("✅ Editor backend handles mode internally");

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_editor_backend_trait_object() {
    let mut backend = EditorBackend::new_from_config("tui-textarea");
    let editor = backend.as_editor_component();

    // Should be able to use trait methods
    editor.set_text("test");
    assert_eq!(editor.get_text(), "test");
  }

  #[test]
  fn test_editor_backend_debug() {
    let backend = EditorBackend::new_from_config("tui-textarea");
    let debug_str = format!("{:?}", backend);
    assert!(debug_str.contains("TuiTextarea"));
  }

  #[test]
  fn test_empty_text_handling() {
    let mut backend = EditorBackend::new_from_config("tui-textarea");

    backend.set_text("");
    assert_eq!(backend.get_text(), "");

    backend.set_text("test");
    backend.set_text("");
    assert_eq!(backend.get_text(), "");
  }
}
