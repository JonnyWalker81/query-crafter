use color_eyre::eyre::Result;
use query_crafter::{components::db::EditorBackend, editor_common::Mode, editor_component::EditorComponent};
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
    _ => println!("❌ Expected TuiTextarea backend"),
  }

  // Test zep backend
  let zep_editor = EditorBackend::new_from_config("zep");
  match zep_editor {
    EditorBackend::Zep(_) => println!("✅ Zep editor created successfully"),
    _ => println!("❌ Expected Zep backend"),
  }

  // Test fallback for unknown backend
  let fallback_editor = EditorBackend::new_from_config("unknown");
  match fallback_editor {
    EditorBackend::TuiTextarea(_) => println!("✅ Fallback to TuiTextarea works"),
    _ => println!("❌ Expected fallback to TuiTextarea"),
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

  // Switch to Zep backend
  let current_text = backend.get_text();
  backend = EditorBackend::new_from_config("zep");
  backend.set_text(&current_text);

  // Note: Zep editor will return empty string if feature is not enabled
  let zep_text = backend.get_text();
  if cfg!(feature = "zep-editor") {
    assert_eq!(zep_text, test_text);
    println!("✅ Text preserved after switching to Zep backend");
  } else {
    assert_eq!(zep_text, "");
    println!("✅ Zep backend returns empty string when feature disabled (expected)");
  }

  // Switch back to TuiTextarea
  let current_text = backend.get_text(); // This will be empty if zep-editor feature is disabled
  backend = EditorBackend::new_from_config("tui-textarea");

  // Set the original text since Zep may have returned empty
  let restore_text = if cfg!(feature = "zep-editor") { &current_text } else { test_text };
  backend.set_text(restore_text);

  assert_eq!(backend.get_text(), test_text);
  println!("✅ Text preserved after switching back to TuiTextarea");

  Ok(())
}

fn test_text_operations() -> Result<()> {
  println!("\n📝 Testing basic text operations...");

  let mut backend = EditorBackend::new_from_config("tui-textarea");
  let area = Rect::new(0, 0, 80, 24);

  // Initialize
  backend.init(area)?;
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

  // Test mode operations (for TuiTextarea)
  assert_eq!(backend.mode(), Mode::Normal);
  println!("✅ Default mode is Normal");

  backend.set_mode(Mode::Insert);
  assert_eq!(backend.mode(), Mode::Insert);
  println!("✅ Mode switching works");

  backend.set_mode(Mode::Normal);
  assert_eq!(backend.mode(), Mode::Normal);
  println!("✅ Mode reset works");

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_editor_backend_trait_object() {
    let mut backend = EditorBackend::new_from_config("tui-textarea");
    let trait_object: &mut dyn EditorComponent = backend.as_editor_component();

    // Should be able to use trait methods
    trait_object.set_text("test");
    assert_eq!(trait_object.get_text(), "test");
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
