use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

/// A single hotkey hint: the key combo and what it does.
pub struct Hotkey {
    pub key: &'static str,
    pub desc: &'static str,
}

impl Hotkey {
    pub const fn new(key: &'static str, desc: &'static str) -> Self {
        Self { key, desc }
    }
}

/// Any component that can report its available hotkeys.
pub trait HotkeyProvider {
    fn hotkeys(&self) -> Vec<Hotkey>;
}

pub fn render(f: &mut Frame, hotkeys: &[Hotkey], area: Rect) {
    let mut spans: Vec<Span> = Vec::new();

    for (i, hk) in hotkeys.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
        }
        spans.push(Span::styled(hk.key, Style::default().fg(Color::Yellow)));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(hk.desc, Style::default().fg(Color::Gray)));
    }

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}
