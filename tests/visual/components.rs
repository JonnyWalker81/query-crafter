use insta::{assert_snapshot, with_settings};
use crate::test_utils::{fixtures, assertions::*};
use query_crafter::components::vim::Vim;
use query_crafter::editor_common::Mode;
use query_crafter::editor_component::EditorComponent;
use ratatui::{
    backend::TestBackend,
    Terminal,
    layout::{Layout, Direction, Constraint, Rect},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    style::{Style, Color},
    text::{Line, Span},
};

#[test]
fn test_vim_editor_normal_mode() {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    
    let mut vim = Vim::new(Mode::Normal);
    vim.set_text("SELECT * FROM users\nWHERE active = true");
    
    terminal.draw(|f| {
        use query_crafter::editor_component::EditorComponent;
        vim.draw(f, f.area());
    }).unwrap();
    
    let output = format!("{}", terminal.backend());
    
    with_settings!({
        description => "Vim editor in normal mode"
    }, {
        assert_snapshot!(output);
    });
}

#[test]
fn test_vim_editor_insert_mode() {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    
    let mut vim = Vim::new(Mode::Insert);
    vim.set_text("SELECT * FROM users");
    
    terminal.draw(|f| {
        use query_crafter::editor_component::EditorComponent;
        vim.draw(f, f.area());
    }).unwrap();
    
    let output = format!("{}", terminal.backend());
    
    with_settings!({
        description => "Vim editor in insert mode"
    }, {
        assert_snapshot!(output);
    });
}

#[test]
fn test_vim_editor_visual_mode() {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    
    let mut vim = Vim::new(Mode::Visual);
    vim.set_text("SELECT * FROM users");
    
    terminal.draw(|f| {
        use query_crafter::editor_component::EditorComponent;
        vim.draw(f, f.area());
    }).unwrap();
    
    let output = format!("{}", terminal.backend());
    
    with_settings!({
        description => "Vim editor in visual mode"
    }, {
        assert_snapshot!(output);
    });
}

#[test]
fn test_table_list_rendering() {
    let backend = TestBackend::new(30, 15);
    let mut terminal = Terminal::new(backend).unwrap();
    
    let tables = fixtures::sample_tables();
    let items: Vec<ListItem> = tables
        .iter()
        .map(|t| ListItem::new(t.name.as_str()))
        .collect();
    
    terminal.draw(|f| {
        let list = List::new(items)
            .block(Block::default()
                .title("Tables")
                .borders(Borders::ALL))
            .highlight_style(Style::default().fg(Color::Yellow));
        f.render_widget(list, f.area());
    }).unwrap();
    
    let output = format!("{}", terminal.backend());
    
    with_settings!({
        description => "Table list with sample tables"
    }, {
        assert_snapshot!(output);
    });
}

#[test]
fn test_results_table_rendering() {
    let backend = TestBackend::new(60, 15);
    let mut terminal = Terminal::new(backend).unwrap();
    
    let (headers, results) = fixtures::sample_query_results();
    
    terminal.draw(|f| {
        use ratatui::widgets::{Table, Row, Cell};
        
        let header = Row::new(headers.iter().map(|h| Cell::from(h.as_str())));
        let rows: Vec<Row> = results
            .iter()
            .map(|row| {
                Row::new(row.iter().map(|cell| Cell::from(cell.as_str())))
            })
            .collect();
        
        let table = Table::new(rows, &[Constraint::Percentage(33); 3])
            .header(header)
            .block(Block::default()
                .title("Query Results")
                .borders(Borders::ALL));
        
        f.render_widget(table, f.area());
    }).unwrap();
    
    let output = format!("{}", terminal.backend());
    
    with_settings!({
        description => "Results table with sample data"
    }, {
        assert_snapshot!(output);
    });
}

#[test]
fn test_error_popup_rendering() {
    let backend = TestBackend::new(50, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    
    terminal.draw(|f| {
        // Main content
        let main_block = Block::default()
            .title("Main Content")
            .borders(Borders::ALL);
        f.render_widget(main_block, f.area());
        
        // Error popup overlay
        let popup_area = centered_rect(60, 20, f.area());
        let error_msg = "Error: Connection to database failed\nPlease check your credentials";
        
        let popup = Paragraph::new(error_msg)
            .block(Block::default()
                .title("Error")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Red)));
        
        f.render_widget(ratatui::widgets::Clear, popup_area);
        f.render_widget(popup, popup_area);
    }).unwrap();
    
    let output = format!("{}", terminal.backend());
    
    with_settings!({
        description => "Error popup overlay"
    }, {
        assert_snapshot!(output);
    });
}

#[test]
fn test_layout_splits() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    
    terminal.draw(|f| {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(50),
                Constraint::Percentage(25),
            ])
            .split(f.area());
        
        let left = Block::default()
            .title("Tables (25%)")
            .borders(Borders::ALL);
        f.render_widget(left, chunks[0]);
        
        let middle = Block::default()
            .title("Query Editor (50%)")
            .borders(Borders::ALL);
        f.render_widget(middle, chunks[1]);
        
        let right = Block::default()
            .title("Results (25%)")
            .borders(Borders::ALL);
        f.render_widget(right, chunks[2]);
    }).unwrap();
    
    let output = format!("{}", terminal.backend());
    
    with_settings!({
        description => "Three-panel layout split"
    }, {
        assert_snapshot!(output);
    });
}

#[test]
fn test_help_content_rendering() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    
    terminal.draw(|f| {
        let help_text = vec![
            Line::from(vec![Span::styled("Navigation", Style::default().fg(Color::Yellow))]),
            Line::from(""),
            Line::from("j/k     - Move up/down"),
            Line::from("gg      - Jump to top"),
            Line::from("G       - Jump to bottom"),
            Line::from(""),
            Line::from(vec![Span::styled("Query Editor", Style::default().fg(Color::Yellow))]),
            Line::from(""),
            Line::from("==      - Format entire query"),
            Line::from("=       - Format selection (visual)"),
            Line::from("Ctrl+Enter - Execute query"),
        ];
        
        let help = Paragraph::new(help_text)
            .block(Block::default()
                .title("Help")
                .borders(Borders::ALL));
        
        f.render_widget(help, f.area());
    }).unwrap();
    
    let output = format!("{}", terminal.backend());
    
    with_settings!({
        description => "Help content display"
    }, {
        assert_snapshot!(output);
    });
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}