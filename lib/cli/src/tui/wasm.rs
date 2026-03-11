use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::{Color, Style},
    widgets::{Block, Borders},
};
use ratatui_code_editor::editor::Editor;

use super::PipelineState;

pub struct CodeState {
    pub editing: bool,
    pub area: Rect,
    pub editor: Editor,
    pub selected_step: usize,
}

pub fn render(f: &mut Frame, pipeline: &PipelineState, state: &mut CodeState, area: Rect) {
    let step = &pipeline.steps[state.selected_step];

    let src_path = step.path.join("src/lib.rs");
    let src_label = src_path.file_name().unwrap_or_default().to_string_lossy();

    let border_color = if state.editing {
        Color::Green
    } else {
        Color::Cyan
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Code: {} ─ {} ", step.name, src_label))
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    f.render_widget(block, area);

    state.area = inner;
    f.render_widget(&state.editor, inner);

    if state.editing {
        if let Some((x, y)) = state.editor.get_visible_cursor(&inner) {
            f.set_cursor_position(Position::new(x, y));
        }
    }
}

pub fn handle_event(state: &mut CodeState, key: KeyEvent) -> anyhow::Result<()> {
    if state.editing {
        state.editor.input(key, &state.area)?;
    }
    Ok(())
}
