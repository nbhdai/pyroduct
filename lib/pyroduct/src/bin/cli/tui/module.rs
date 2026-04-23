use crossterm::event::{KeyCode, KeyEvent};
use pyro_artifacts::cache::CacheManager;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::Tabs,
};

use super::{
    cap_config::CapConfigState,
    keys::{Hotkey, HotkeyProvider},
    logs::LogsView,
    wasm,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePane {
    Code,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottomTab {
    Config,
    Logs,
}

pub struct ModuleView {
    pub code: wasm::CodeState,
    pub cap_config: CapConfigState,
    pub logs: LogsView,
    pub active_pane: ActivePane,
    pub bottom_tab: BottomTab,
    pub focused: bool,
}

impl ModuleView {
    pub fn new(code: wasm::CodeState, cap_config: CapConfigState) -> Self {
        Self {
            code,
            cap_config,
            logs: LogsView::default(),
            active_pane: ActivePane::Code,
            bottom_tab: BottomTab::Config,
            focused: false,
        }
    }

    pub fn selected_step(&self) -> usize {
        self.code.selected_step
    }

    pub fn render(&mut self, f: &mut Frame<'_>, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
            .split(area);

        // Code pane
        let orig_editing = self.code.editing;
        self.code.editing = self.focused && self.active_pane == ActivePane::Code;
        wasm::render(f, &mut self.code, chunks[0]);
        self.code.editing = orig_editing;

        // Bottom pane: tab bar + content
        let bottom_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(chunks[1]);

        let tab_idx = match self.bottom_tab {
            BottomTab::Config => 0,
            BottomTab::Logs => 1,
        };
        let tabs = Tabs::new(vec![Line::from("Config"), Line::from("Logs")])
            .select(tab_idx)
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Yellow),
            )
            .divider(" │ ");
        f.render_widget(tabs, bottom_chunks[0]);

        let is_bottom_focused = self.focused && self.active_pane == ActivePane::Bottom;

        match self.bottom_tab {
            BottomTab::Config => {
                self.cap_config.active = is_bottom_focused;
                self.cap_config.render(f, bottom_chunks[1], "Cap Config");
            }
            BottomTab::Logs => {
                self.logs
                    .render(f, bottom_chunks[1], "Compiler Output", is_bottom_focused);
            }
        }
    }

    pub async fn handle_event(
        &mut self,
        key: KeyEvent,
        cache: &CacheManager,
    ) -> anyhow::Result<()> {
        // Tab switches between code and bottom pane
        if key.code == KeyCode::Tab {
            match self.active_pane {
                ActivePane::Code => self.code.editing = false,
                ActivePane::Bottom => self.cap_config.editing = false,
            }
            self.active_pane = match self.active_pane {
                ActivePane::Code => ActivePane::Bottom,
                ActivePane::Bottom => ActivePane::Code,
            };
            if self.active_pane == ActivePane::Code {
                self.code.editing = true;
            }
            return Ok(());
        }

        // Backtab (Shift+Tab) switches bottom tabs when bottom pane is active
        if key.code == KeyCode::BackTab && self.active_pane == ActivePane::Bottom {
            self.cap_config.editing = false;
            self.bottom_tab = match self.bottom_tab {
                BottomTab::Config => BottomTab::Logs,
                BottomTab::Logs => {
                    // Switching TO Config
                    self.cap_config.refresh_available_caps(cache).await;
                    if self.cap_config.selected_tab < self.cap_config.editors.len() {
                        let name = self.cap_config.editors[self.cap_config.selected_tab]
                            .0
                            .clone();
                        self.cap_config.load_interface(cache, &name).await;
                    }
                    BottomTab::Config
                }
            };
            return Ok(());
        }

        match (self.active_pane, &key.code) {
            (ActivePane::Code, KeyCode::Esc) => {
                if self.code.editing {
                    self.code.editing = false;
                } else {
                    self.focused = false;
                }
            }
            (ActivePane::Code, _) => wasm::handle_event(&mut self.code, key)?,
            (ActivePane::Bottom, KeyCode::Esc) => match self.bottom_tab {
                BottomTab::Config => {
                    if self.cap_config.editing {
                        self.cap_config.editing = false;
                    } else {
                        self.focused = false;
                    }
                }
                BottomTab::Logs => {
                    self.focused = false;
                }
            },
            (ActivePane::Bottom, _) => match self.bottom_tab {
                BottomTab::Config => self.cap_config.handle_event(key, cache).await?,
                BottomTab::Logs => self.logs.handle_event(key),
            },
        }
        Ok(())
    }
}

impl HotkeyProvider for ModuleView {
    fn hotkeys(&self) -> Vec<Hotkey> {
        let mut hk = vec![Hotkey::new("Tab", "Switch pane")];
        match self.active_pane {
            ActivePane::Code => hk.push(Hotkey::new("...", "Code editor")),
            ActivePane::Bottom => {
                hk.push(Hotkey::new("S-Tab", "Switch tab"));
                match self.bottom_tab {
                    BottomTab::Config => hk.extend(self.cap_config.hotkeys()),
                    BottomTab::Logs => hk.extend(self.logs.hotkeys()),
                }
            }
        }
        hk
    }
}
