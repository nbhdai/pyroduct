// lib/cli/src/tui/cap_config.rs
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};
use ratatui_code_editor::{editor::Editor, theme::vesper};
use super::keys::{Hotkey, HotkeyProvider};

pub struct CapConfigState {
    pub editing: bool,
    pub area: Rect,
    pub editors: Vec<(String, Editor)>, // (Capability Class Name, Editor)
    pub selected_tab: usize,
}

impl CapConfigState {
    pub fn new(configs: Vec<(String, String)>) -> Self {
        let mut editors = Vec::new();
        for (name, yaml) in configs {
            editors.push((name, Editor::new("yaml", &yaml, vesper())));
        }
        
        if editors.is_empty() {
            editors.push(("DefaultCapability".to_string(), Editor::new("yaml", "", vesper())));
        }
        
        Self {
            editing: false,
            area: Rect::default(),
            editors,
            selected_tab: 0,
        }
    }

    pub fn handle_event(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        if self.editing {
            // Forward input to the active editor
            self.editors[self.selected_tab].1.input(key, &self.area)?;
        } else {
            // Navigation mode (Tab switching & creation)
            match key.code {
                KeyCode::Left | KeyCode::Char('h') => {
                    if self.selected_tab > 0 {
                        self.selected_tab -= 1;
                    }
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    if self.selected_tab < self.editors.len() {
                        self.selected_tab += 1;
                    }
                }
                KeyCode::Enter | KeyCode::Char('i') => {
                    if self.selected_tab == self.editors.len() {
                        // User clicked Enter on the "+" tab to add a new capability
                        let new_name = format!("NewCapability{}", self.editors.len() + 1);
                        self.editors.push((new_name, Editor::new("yaml", "", vesper())));
                        self.selected_tab = self.editors.len() - 1;
                    } else {
                        // Focus on the current editor
                        self.editing = true;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect, title: &str) {
        let border_color = if self.editing { Color::Green } else { Color::Cyan };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", title))
            .border_style(Style::default().fg(border_color));

        let inner_area = block.inner(area);
        f.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(0)])
            .split(inner_area);

        // Build the tab titles: [Cap1] [Cap2] [+]
        let mut tab_titles: Vec<Line> = self
            .editors
            .iter()
            .map(|(name, _)| Line::from(name.as_str()))
            .collect();
        tab_titles.push(Line::from("+"));

        let tabs = Tabs::new(tab_titles)
            .select(self.selected_tab)
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Yellow),
            )
            .divider(" │ ");

        f.render_widget(tabs, chunks[0]);

        // Render Editor or "Create" placeholder
        if self.selected_tab < self.editors.len() {
            let editor_area = chunks[1];
            self.area = editor_area; // save area for handle_event math
            f.render_widget(&self.editors[self.selected_tab].1, editor_area);

            // Display cursor if actively editing
            if self.editing {
                if let Some((x, y)) = self.editors[self.selected_tab].1.get_visible_cursor(&editor_area) {
                    f.set_cursor_position(Position::new(x, y));
                }
            }
        } else {
            // Selected the "+" tab but haven't pressed Enter yet
            let add_msg = Paragraph::new("Press Enter to add a new capability config")
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(add_msg, chunks[1]);
        }
    }
}

impl HotkeyProvider for CapConfigState {
    fn hotkeys(&self) -> Vec<Hotkey> {
        if self.editing {
            vec![Hotkey::new("Esc", "Stop editing")]
        } else {
            vec![
                Hotkey::new("←/→", "Switch tab"),
                Hotkey::new("Enter", "Edit"),
            ]
        }
    }
}