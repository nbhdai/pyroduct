use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Position, Rect},
    style::{Color, Style},
    widgets::{Block, Borders},
    Frame,
};

use super::App;

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let step = &app.steps[app.selected_step];
    
    // Resolve the display name of the source file
    let src_path = step.path.join("src/lib.rs");
    let src_label = src_path.file_name().unwrap_or_default().to_string_lossy();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Code: {} ─ {} ", step.name, src_label))
        .border_style(Style::default().fg(Color::Cyan));
    
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Save the inner area so we know where to route mouse/cursor events later
    app.code_area = inner;
    f.render_widget(&app.steps[app.selected_step].code, inner);

    // Ensure the terminal cursor is physically placed where the editor cursor is
    if app.editing {
        if let Some((x, y)) = app.steps[app.selected_step].code.get_visible_cursor(&inner) {
            f.set_cursor_position(Position::new(x, y));
        }
    }
}

pub fn handle_event(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    if key.code == KeyCode::Esc {
        app.editing = false;
        app.status_msg = "Navigation mode.".into();
        return Ok(());
    }

    let area = app.code_area;
    app.steps[app.selected_step].code.input(key, &area)?;
    Ok(())
}