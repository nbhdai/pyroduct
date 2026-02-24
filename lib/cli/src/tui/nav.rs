use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use super::{App, ViewState};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    // Map the pipeline steps to Ratatui ListItems
    let items: Vec<ListItem> = app
        .pipeline
        .steps
        .iter()
        .map(|step| ListItem::new(step.name.clone()))
        .collect();

    // Configure the List widget
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Steps ")
                .border_style(Style::default().fg(Color::Gray)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    // Determine the selected step from the active view
    let selected_step = match &app.view {
        ViewState::Code(state) => state.selected_step,
    };

    // Set the active selection
    let mut state = ListState::default();
    state.select(Some(selected_step));

    // Render the stateful list
    f.render_stateful_widget(list, area, &mut state);
}