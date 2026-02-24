use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Position, Rect},
    style::{Color, Style},
    widgets::{Block, Borders},
    Frame,
};
use ratatui_code_editor::{editor::Editor, theme::vesper};

use super::PipelineState;

/// The isolated state for the Code view
pub struct CodeState {
    pub editing: bool,
    pub area: Rect,
    pub editor: Editor,
    pub selected_step: usize,
    pub status_msg: String,
    pub quit: bool,
}

pub fn render(f: &mut Frame, pipeline: &PipelineState, state: &mut CodeState, area: Rect) {
    let step = &pipeline.steps[state.selected_step];

    let src_path = step.path.join("src/lib.rs");
    let src_label = src_path.file_name().unwrap_or_default().to_string_lossy();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Code: {} ─ {} ", step.name, src_label))
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Save the area and render the editor
    state.area = inner;
    f.render_widget(&state.editor, inner);

    // Ensure the terminal cursor matches the editor cursor
    if state.editing {
        if let Some((x, y)) = state.editor.get_visible_cursor(&inner) {
            f.set_cursor_position(Position::new(x, y));
        }
    }
}

pub fn handle_event(
    pipeline: &mut PipelineState,
    state: &mut CodeState,
    key: KeyEvent,
) -> anyhow::Result<()> {
    if state.editing {
        if key.code == KeyCode::Esc {
            state.editing = false;
            state.status_msg = "Navigation mode.".into();
        } else {
            // Forward input straight to the editor
            state.editor.input(key, &state.area)?;
        }
    } else {
        match key.code {
            KeyCode::Enter | KeyCode::Char('i') => {
                state.editing = true;
                state.status_msg = "Editing ─ Esc to return".into();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if state.selected_step > 0 {
                    sync_and_switch(pipeline, state, state.selected_step - 1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if state.selected_step + 1 < pipeline.steps.len() {
                    sync_and_switch(pipeline, state, state.selected_step + 1);
                }
            }
            _ => {}
        }
    }

    Ok(())
}

/// Commits the active editor's text to the old module, changes the index,
/// and rehydrates the editor with the new module's text.
fn sync_and_switch(pipeline: &mut PipelineState, state: &mut CodeState, new_step_idx: usize) {
    // Save out the old string
    pipeline.steps[state.selected_step].source_code = state.editor.get_content();
    // Change selection
    state.selected_step = new_step_idx;
    // Hydrate the editor with the new module's string
    state.editor = Editor::new("rust", &pipeline.steps[new_step_idx].source_code, vesper());
}