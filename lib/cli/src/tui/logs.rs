use crossterm::event::{KeyCode, KeyEvent};
use pyroduct::pipeline::wasm::PyroLogs;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct LogsView {
    pub logs: PyroLogs,
    scroll: u16,
    formatted_lines: Vec<Line<'static>>,
}

impl LogsView {
    pub fn default() -> Self {
        Self {
            logs: PyroLogs::empty(),
            scroll: 0,
            formatted_lines: Vec::new(),
        }
    }
    pub fn new(logs: PyroLogs) -> Self {
        let mut lines = Vec::new();

        // Flatten module logs
        for log in &logs.module_logs {
            lines.push(Line::from(vec![
                Span::styled("[Module] ", Style::default().fg(Color::Cyan)),
                Span::raw(log.clone()),
            ]));
        }

        // Flatten capability logs
        for ((lib, cap), cap_logs) in &logs.capability_logs {
            let prefix = format!("[{}::{}] ", lib, cap);
            for log in cap_logs {
                lines.push(Line::from(vec![
                    Span::styled(prefix.clone(), Style::default().fg(Color::Yellow)),
                    Span::raw(log.clone()),
                ]));
            }
        }

        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                "No logs for this step.",
                Style::default().fg(Color::DarkGray),
            )));
        }

        Self {
            logs,
            scroll: 0,
            formatted_lines: lines,
        }
    }

    pub fn scroll_up(&mut self, amount: u16) {
        self.scroll = self.scroll.saturating_sub(amount);
    }

    pub fn scroll_down(&mut self, amount: u16) {
        let max_scroll = self.formatted_lines.len().saturating_sub(1) as u16;
        self.scroll = self.scroll.saturating_add(amount).min(max_scroll);
    }

    pub fn home(&mut self) {
        self.scroll = 0;
    }

    pub fn end(&mut self) {
        self.scroll = self.formatted_lines.len().saturating_sub(1) as u16;
    }

    pub fn handle_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.scroll_up(1),
            KeyCode::Down | KeyCode::Char('j') => self.scroll_down(1),
            KeyCode::PageUp => self.scroll_up(10),
            KeyCode::PageDown => self.scroll_down(10),
            KeyCode::Home => self.home(),
            KeyCode::End => self.end(),
            _ => {}
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect, title: &str, focused: bool) {
        let border_color = if focused { Color::Green } else { Color::Cyan };
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", title))
            .border_style(Style::default().fg(border_color));

        let paragraph = Paragraph::new(self.formatted_lines.clone())
            .block(block)
            .scroll((self.scroll, 0));

        f.render_widget(paragraph, area);
    }
}