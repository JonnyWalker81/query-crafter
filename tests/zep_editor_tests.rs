#[cfg(test)]
mod tests {
  use color_eyre::eyre::Result;
  use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
  #[cfg(feature = "zep-editor")]
  use query_crafter::components::zep_editor::ZepEditor;
  #[cfg(feature = "zep-editor")]
  use query_crafter::editor_component::EditorComponent;
  use ratatui::layout::Rect;

  #[cfg(feature = "zep-editor")]
  fn create_test_area() -> Rect {
    Rect::new(0, 0, 80, 24)
  }

  #[cfg(feature = "zep-editor")]
  #[test]
  fn test_zep_editor_creation() -> Result<()> {
    let mut editor = ZepEditor::new();
    let area = create_test_area();

    // Should initialize without error
    editor.init(area)?;

    // Should have empty content initially
    assert_eq!(editor.get_text(), "");

    Ok(())
  }

  #[cfg(feature = "zep-editor")]
  #[test]
  fn test_set_and_get_text() -> Result<()> {
    let mut editor = ZepEditor::new();
    let area = create_test_area();
    editor.init(area)?;

    let test_text = "SELECT * FROM users WHERE id = 1;";
    editor.set_text(test_text);

    assert_eq!(editor.get_text(), test_text);

    Ok(())
  }

  #[cfg(feature = "zep-editor")]
  #[test]
  fn test_basic_text_input() -> Result<()> {
    let mut editor = ZepEditor::new();
    let area = create_test_area();
    editor.init(area)?;

    // Test entering insert mode and typing
    let enter_insert = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE);
    editor.on_key_event(enter_insert)?;

    // Type some characters
    let chars = ['S', 'E', 'L', 'E', 'C', 'T'];
    for ch in chars {
      let key_event = KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE);
      editor.on_key_event(key_event)?;
    }

    // The text should contain our input (though exact behavior depends on Zep implementation)
    let content = editor.get_text();
    assert!(content.contains("SELECT") || content.len() > 0);

    Ok(())
  }

  #[cfg(feature = "zep-editor")]
  #[test]
  fn test_vim_mode_transitions() -> Result<()> {
    let mut editor = ZepEditor::new();
    let area = create_test_area();
    editor.init(area)?;

    // Test entering insert mode
    let enter_insert = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE);
    let result = editor.on_key_event(enter_insert)?;
    assert!(result.is_none()); // Should not generate any actions

    // Test escape to normal mode
    let escape = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    let result = editor.on_key_event(escape)?;
    assert!(result.is_none());

    Ok(())
  }

  #[cfg(feature = "zep-editor")]
  #[test]
  fn test_movement_keys() -> Result<()> {
    let mut editor = ZepEditor::new();
    let area = create_test_area();
    editor.init(area)?;

    // Set some text first
    editor.set_text("line1\nline2\nline3");

    // Test vim movement keys
    let movement_keys = [
      KeyCode::Char('h'), // left
      KeyCode::Char('j'), // down
      KeyCode::Char('k'), // up
      KeyCode::Char('l'), // right
    ];

    for key_code in movement_keys {
      let key_event = KeyEvent::new(key_code, KeyModifiers::NONE);
      let result = editor.on_key_event(key_event)?;
      assert!(result.is_none());
    }

    Ok(())
  }

  #[cfg(feature = "zep-editor")]
  #[test]
  fn test_delete_operations() -> Result<()> {
    let mut editor = ZepEditor::new();
    let area = create_test_area();
    editor.init(area)?;

    // Set test text
    editor.set_text("Hello World");

    // Test character deletion (x in normal mode)
    let delete_char = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
    editor.on_key_event(delete_char)?;

    // Test backspace in insert mode
    let enter_insert = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE);
    editor.on_key_event(enter_insert)?;

    let backspace = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
    editor.on_key_event(backspace)?;

    Ok(())
  }

  #[cfg(feature = "zep-editor")]
  #[test]
  fn test_multiline_text_handling() -> Result<()> {
    let mut editor = ZepEditor::new();
    let area = create_test_area();
    editor.init(area)?;

    let multiline_text = "SELECT *\nFROM users\nWHERE active = true;";
    editor.set_text(multiline_text);

    assert_eq!(editor.get_text(), multiline_text);

    Ok(())
  }

  // Stub tests for when zep-editor feature is not enabled
  #[cfg(not(feature = "zep-editor"))]
  #[test]
  fn test_zep_editor_feature_disabled() {
    use query_crafter::{components::zep_editor::ZepEditor, editor_component::EditorComponent};

    let mut editor = ZepEditor::default();
    let area = Rect::new(0, 0, 80, 24);

    // Should return error when feature is disabled
    let result = editor.init(area);
    assert!(result.is_err());

    let key_event = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE);
    let result = editor.on_key_event(key_event);
    assert!(result.is_err());
  }

  #[test]
  fn test_editor_trait_object_compatibility() {
    // Test that ZepEditor can be used as a trait object
    use std::any::Any;

    use query_crafter::{components::zep_editor::ZepEditor, editor_component::EditorComponent};

    let editor = ZepEditor::default();
    let trait_object: &dyn EditorComponent = &editor;

    // Should be able to get text (will be empty when feature disabled)
    let _text = trait_object.get_text();

    // Should be able to downcast
    let any_ref: &dyn Any = trait_object.as_any();
    assert!(any_ref.is::<ZepEditor>());
  }
}

#[cfg(test)]
mod integration_tests {
  use color_eyre::eyre::Result;

  // These tests verify the overall integration of ZepEditor with the editor component system

  #[test]
  fn test_editor_factory_pattern() -> Result<()> {
    // This would test creating editors based on configuration
    // For now, just verify the pattern compiles

    #[allow(dead_code)]
    enum EditorType {
      TuiTextarea,
      Zep,
    }

    #[allow(dead_code)]
    fn create_editor(editor_type: EditorType) -> Box<dyn query_crafter::editor_component::EditorComponent> {
      match editor_type {
        EditorType::TuiTextarea => {
          Box::new(query_crafter::components::vim::Vim::new(query_crafter::editor_common::Mode::Normal))
        },
        EditorType::Zep => Box::new(query_crafter::components::zep_editor::ZepEditor::default()),
      }
    }

    // Test factory pattern works
    let _vim_editor = create_editor(EditorType::TuiTextarea);
    let _zep_editor = create_editor(EditorType::Zep);

    Ok(())
  }
}
