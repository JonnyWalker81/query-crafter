use query_crafter::components::db::DbTable;

pub trait StateAssertions {
    fn assert_mode(&self, expected: &str);
    fn assert_selected_table(&self, expected: &str);
    fn assert_query_text(&self, expected: &str);
    fn assert_has_results(&self);
    fn assert_no_results(&self);
    fn assert_error(&self, expected: &str);
    fn assert_no_error(&self);
    fn assert_component_focused(&self, component: &str);
}

pub trait RenderAssertions {
    fn assert_contains(&self, text: &str);
    fn assert_not_contains(&self, text: &str);
    fn assert_line_contains(&self, line: usize, text: &str);
    fn assert_visible_table(&self, table_name: &str);
    fn assert_help_visible(&self);
    fn assert_popup_visible(&self);
}

impl RenderAssertions for Vec<String> {
    fn assert_contains(&self, text: &str) {
        let full_content = self.join("\n");
        assert!(
            full_content.contains(text),
            "Expected to find '{}' in rendered output:\n{}",
            text,
            full_content
        );
    }
    
    fn assert_not_contains(&self, text: &str) {
        let full_content = self.join("\n");
        assert!(
            !full_content.contains(text),
            "Expected NOT to find '{}' in rendered output:\n{}",
            text,
            full_content
        );
    }
    
    fn assert_line_contains(&self, line: usize, text: &str) {
        assert!(
            line < self.len(),
            "Line {} is out of bounds (total lines: {})",
            line,
            self.len()
        );
        
        assert!(
            self[line].contains(text),
            "Expected line {} to contain '{}', but got: '{}'",
            line,
            text,
            self[line]
        );
    }
    
    fn assert_visible_table(&self, table_name: &str) {
        let found = self.iter().any(|line| {
            line.contains(table_name) && !line.trim().is_empty()
        });
        
        assert!(
            found,
            "Expected table '{}' to be visible in output",
            table_name
        );
    }
    
    fn assert_help_visible(&self) {
        self.assert_contains("Keyboard Shortcuts");
    }
    
    fn assert_popup_visible(&self) {
        // Check for common popup indicators like borders
        let has_popup = self.iter().any(|line| {
            line.contains("┌") || line.contains("└") || line.contains("│")
        });
        
        assert!(
            has_popup,
            "Expected a popup to be visible (looking for border characters)"
        );
    }
}

// Helper functions for common assertions
pub fn assert_vim_mode(buffer: &[String], mode: &str) {
    let mode_line = buffer.iter()
        .find(|line| line.contains("Query Editor"))
        .expect("Could not find Query Editor mode line");
    
    assert!(
        mode_line.contains(mode),
        "Expected vim mode '{}' but found: {}",
        mode,
        mode_line
    );
}

pub fn assert_query_content(buffer: &[String], expected: &str) {
    // Find the query editor area (usually between borders)
    let mut in_editor = false;
    let mut content = String::new();
    
    for line in buffer {
        if line.contains("Query Editor") {
            in_editor = true;
            continue;
        }
        
        if in_editor && (line.contains("─") || line.contains("Results")) {
            break;
        }
        
        if in_editor && !line.trim().is_empty() {
            content.push_str(line.trim());
            content.push('\n');
        }
    }
    
    let content = content.trim();
    assert_eq!(
        content, expected,
        "Query content mismatch.\nExpected:\n{}\nActual:\n{}",
        expected, content
    );
}

pub fn assert_selected_row(buffer: &[String], row_content: &str) {
    // Look for highlighted row (usually with different styling)
    let found = buffer.iter().any(|line| {
        line.contains(row_content)
    });
    
    assert!(
        found,
        "Expected to find selected row with content: {}",
        row_content
    );
}

pub fn assert_table_count(tables: &[DbTable], expected: usize) {
    assert_eq!(
        tables.len(),
        expected,
        "Expected {} tables but found {}",
        expected,
        tables.len()
    );
}

pub fn assert_results_count(results: &[Vec<String>], expected: usize) {
    assert_eq!(
        results.len(),
        expected,
        "Expected {} result rows but found {}",
        expected,
        results.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_render_assertions() {
        let buffer = vec![
            "┌─────────────┐".to_string(),
            "│ Test Window │".to_string(),
            "└─────────────┘".to_string(),
            "Hello World".to_string(),
        ];
        
        buffer.assert_contains("Test Window");
        buffer.assert_not_contains("Not Present");
        buffer.assert_line_contains(3, "Hello World");
        buffer.assert_popup_visible();
    }
    
    #[test]
    #[should_panic(expected = "Expected to find 'Missing'")]
    fn test_assert_contains_failure() {
        let buffer = vec!["Hello".to_string()];
        buffer.assert_contains("Missing");
    }
}