use super::keys::{Hotkey, HotkeyProvider};
use pyro_artifacts::cache::CacheManager;
use crossterm::event::{KeyCode, KeyEvent};
use pyroduct::format::value::InterfaceSpec;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs, Wrap},
};
use ratatui_code_editor::{editor::Editor, theme::vesper};
use std::collections::HashMap;

pub struct CapConfigState {
    pub editing: bool,
    pub active: bool,
    pub area: Rect,
    pub editors: Vec<(String, Editor)>,
    pub selected_tab: usize,
    pub available_caps: Vec<(String, String, String)>,
    pub add_list_state: ListState,
    pub interfaces: HashMap<String, InterfaceSpec<'static>>,
}

impl CapConfigState {
    pub fn new(configs: Vec<(String, String)>) -> Self {
        let mut editors = Vec::new();
        for (name, yaml) in configs {
            let yaml_str = if yaml.trim().is_empty() {
                String::new()
            } else {
                serde_yaml::to_string(&yaml).unwrap_or(yaml)
            };
            editors.push((name, Editor::new("yaml", &yaml_str, vesper())));
        }

        Self {
            editing: false,
            active: false,
            area: Rect::default(),
            editors,
            selected_tab: 0,
            available_caps: Vec::new(),
            add_list_state: ListState::default(),
            interfaces: HashMap::new(),
        }
    }

    pub async fn refresh_available_caps(&mut self, cache: &CacheManager) {
        if let Ok(caps) = cache.list_available_capabilities().await {
            self.available_caps = caps;
            if self.add_list_state.selected().is_none() && !self.available_caps.is_empty() {
                self.add_list_state.select(Some(0));
            }
        }
    }

    pub async fn load_interface(&mut self, cache: &CacheManager, name: &str) {
        if self.interfaces.contains_key(name) {
            return;
        }

        // Search for this name in available_caps to get author and version
        if let Some((author, cap_name, version)) = self
            .available_caps
            .iter()
            .find(|(_, n, _)| n == name)
            .cloned()
        {
            if let Ok(json) = cache
                .capability_interface_spec(&author, &cap_name, &version)
                .await
            {
                if let Ok(spec) = serde_json::from_str::<InterfaceSpec>(&json) {
                    self.interfaces.insert(name.to_string(), spec);
                }
            }
        }
    }

    pub async fn handle_event(
        &mut self,
        key: KeyEvent,
        cache: &CacheManager,
    ) -> anyhow::Result<()> {
        if self.editing {
            self.editors[self.selected_tab].1.input(key, &self.area)?;
        } else {
            match key.code {
                KeyCode::Left | KeyCode::Char('h') => {
                    if self.selected_tab > 0 {
                        self.selected_tab -= 1;
                        if self.selected_tab < self.editors.len() {
                            let name = self.editors[self.selected_tab].0.clone();
                            self.load_interface(cache, &name).await;
                        }
                    }
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    if self.selected_tab < self.editors.len() {
                        self.selected_tab += 1;
                        if self.selected_tab < self.editors.len() {
                            let name = self.editors[self.selected_tab].0.clone();
                            self.load_interface(cache, &name).await;
                        } else {
                            // Switched to "Add" tab
                            self.refresh_available_caps(cache).await;
                        }
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.selected_tab == self.editors.len() {
                        let i = self.add_list_state.selected().unwrap_or(0);
                        if i > 0 {
                            self.add_list_state.select(Some(i - 1));
                        }
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.selected_tab == self.editors.len() {
                        let i = self.add_list_state.selected().unwrap_or(0);
                        if i + 1 < self.available_caps.len() {
                            self.add_list_state.select(Some(i + 1));
                        }
                    }
                }
                KeyCode::Enter | KeyCode::Char('i') => {
                    if self.selected_tab == self.editors.len() {
                        if let Some(i) = self.add_list_state.selected() {
                            let name = self.available_caps[i].1.clone();
                            self.editors
                                .push((name.clone(), Editor::new("yaml", "", vesper())));
                            self.selected_tab = self.editors.len() - 1;
                            self.load_interface(cache, &name).await;
                        }
                    } else {
                        self.editing = true;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect, title: &str) {
        let border_color = if self.editing {
            Color::Green
        } else if self.active {
            Color::Yellow
        } else {
            Color::Cyan
        };

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

        let mut tab_titles: Vec<Line> = self
            .editors
            .iter()
            .map(|(name, _)| Line::from(name.as_str()))
            .collect();
        tab_titles.push(Line::from("+ Add"));

        let tabs = Tabs::new(tab_titles)
            .select(self.selected_tab)
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Yellow),
            )
            .divider(" │ ");

        f.render_widget(tabs, chunks[0]);

        if self.selected_tab < self.editors.len() {
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[1]);

            let editor_area = main_chunks[0];
            self.area = editor_area;
            f.render_widget(&self.editors[self.selected_tab].1, editor_area);

            if self.editing {
                if let Some((x, y)) = self.editors[self.selected_tab]
                    .1
                    .get_visible_cursor(&editor_area)
                {
                    f.set_cursor_position(Position::new(x, y));
                }
            }

            // Render documentation on the right
            let cap_name = &self.editors[self.selected_tab].0;
            let doc_text = if let Some(spec) = self.interfaces.get(cap_name) {
                render_pseudo_rust(spec)
            } else {
                vec![Line::from(Span::styled(
                    "No documentation available for this capability.",
                    Style::default().fg(Color::DarkGray),
                ))]
            };

            let doc_para = Paragraph::new(doc_text)
                .block(
                    Block::default()
                        .borders(Borders::LEFT)
                        .border_style(Style::default().fg(Color::DarkGray))
                        .title(" Documentation "),
                )
                .wrap(Wrap { trim: false });
            f.render_widget(doc_para, main_chunks[1]);
        } else {
            let add_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(0)])
                .split(chunks[1]);

            f.render_widget(
                Paragraph::new("Select a capability to add:")
                    .style(Style::default().fg(Color::Gray)),
                add_chunks[0],
            );

            let items: Vec<ListItem> = self
                .available_caps
                .iter()
                .map(|(author, name, version)| {
                    ListItem::new(Line::from(vec![
                        Span::styled(name, Style::default().fg(Color::Cyan)),
                        Span::raw(" "),
                        Span::styled(
                            format!("v{}", version),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::raw(" by "),
                        Span::styled(author, Style::default().fg(Color::Gray)),
                    ]))
                })
                .collect();

            let list = List::new(items)
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> ");

            f.render_stateful_widget(list, add_chunks[1], &mut self.add_list_state);
        }
    }
}

fn render_pseudo_rust<'a>(spec: &InterfaceSpec<'a>) -> Vec<Line<'a>> {
    let mut lines = Vec::new();

    if let Some(desc) = &spec.description {
        for line in desc.lines() {
            lines.push(Line::from(vec![Span::styled(
                format!("/// {}", line),
                Style::default().fg(Color::Green),
            )]));
        }
    }

    lines.push(Line::from(vec![
        Span::styled("pub struct ", Style::default().fg(Color::Magenta)),
        Span::styled(spec.capability.clone(), Style::default().fg(Color::Yellow)),
        Span::raw(" {"),
    ]));

    if let Some(class) = spec.classes.first() {
        for method in &class.methods {
            lines.push(Line::from(""));
            if let Some(desc) = &method.description {
                for line in desc.lines() {
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(format!("/// {}", line), Style::default().fg(Color::Green)),
                    ]));
                }
            }

            let mut method_line = vec![
                Span::raw("    "),
                Span::styled("fn ", Style::default().fg(Color::Magenta)),
                Span::styled(method.name.clone(), Style::default().fg(Color::Cyan)),
                Span::raw("("),
            ];

            // This is a simplification, InterfaceSpec doesn't have full rust type info here
            // but it has PyroSchema which we could potentially detail more.
            method_line.push(Span::raw("..."));
            method_line.push(Span::raw(") -> "));
            method_line.push(Span::styled(
                format!("{}", method.output),
                Style::default().fg(Color::Yellow),
            ));
            method_line.push(Span::raw(";"));

            lines.push(Line::from(method_line));
        }
    }

    lines.push(Line::from("}"));

    lines
}

impl HotkeyProvider for CapConfigState {
    fn hotkeys(&self) -> Vec<Hotkey> {
        if self.editing {
            vec![Hotkey::new("Esc", "Stop editing")]
        } else if self.selected_tab == self.editors.len() {
            vec![
                Hotkey::new("←/→", "Switch tab"),
                Hotkey::new("↑/↓", "Select cap"),
                Hotkey::new("Enter", "Add"),
            ]
        } else {
            vec![
                Hotkey::new("←/→", "Switch tab"),
                Hotkey::new("Enter", "Edit"),
            ]
        }
    }
}
