use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use super::App;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    let list_area = chunks[0];
    let run_area = chunks[1];

    // Build items: Input -> Steps -> Output
    let mut items = vec![ListItem::new("Input Table")];
    items.extend(
        app.pipeline
            .pipelines
            .iter()
            .enumerate()
            .map(|(_, p)| ListItem::new(format!("Pipeline: {}", p.name))),
    );
    items.push(ListItem::new("Output Table"));

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Flow ")
                .border_style(Style::default().fg(Color::Gray)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    let mut state = ListState::default();
    state.select(Some(app.current_nav_index()));

    f.render_stateful_widget(list, list_area, &mut state);

    let run_text = Line::from(vec![
        Span::styled("Run ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled("(^R)", Style::default().fg(Color::DarkGray)),
    ]);

    let run_block = Paragraph::new(run_text).alignment(Alignment::Center).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green)),
    );

    f.render_widget(run_block, run_area);
}
