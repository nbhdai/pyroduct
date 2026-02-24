use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use super::{cap_config::CapConfigState, keys::{Hotkey, HotkeyProvider}, wasm, PipelineState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePane {
    Code,
    CapConfig,
}

pub struct ModuleView {
    pub code: wasm::CodeState,
    pub cap_config: CapConfigState,
    pub active_pane: ActivePane,
    pub focused: bool,
}

impl ModuleView {
    pub fn new(code: wasm::CodeState, cap_config: CapConfigState) -> Self {
        Self {
            code,
            cap_config,
            active_pane: ActivePane::Code,
            focused: false,
        }
    }

    pub fn selected_step(&self) -> usize {
        self.code.selected_step
    }

    pub fn render(&mut self, f: &mut Frame, pipeline: &PipelineState, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
            .split(area);

        let orig_editing = self.code.editing;
        self.code.editing = self.focused && self.active_pane == ActivePane::Code;
        wasm::render(f, pipeline, &mut self.code, chunks[0]);
        self.code.editing = orig_editing;

        let cap_editing = self.focused && self.active_pane == ActivePane::CapConfig;
        self.cap_config.editing = cap_editing;
        self.cap_config.render(f, chunks[1], "Cap Config");
    }

    pub fn handle_event(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        if key.code == KeyCode::Tab {
            match self.active_pane {
                ActivePane::Code => self.code.editing = false,
                ActivePane::CapConfig => self.cap_config.editing = false,
            }
            self.active_pane = match self.active_pane {
                ActivePane::Code => ActivePane::CapConfig,
                ActivePane::CapConfig => ActivePane::Code,
            };
            // Code pane enters editing immediately; cap_config starts in
            // tab-navigation mode so the user can reach the "+" tab first.
            if self.active_pane == ActivePane::Code {
                self.code.editing = true;
            }
            return Ok(());
        }

        match (self.active_pane, &key.code) {
            (ActivePane::Code, KeyCode::Esc) => {
                if self.code.editing == true {
                    self.code.editing = false;
                } else {
                    self.focused = false;
                }
            },
            (ActivePane::Code, _) => wasm::handle_event(&mut self.code, key)?,
            (ActivePane::CapConfig, KeyCode::Esc) => {
                if self.cap_config.editing == true {
                    self.cap_config.editing = false;
                } else {
                    self.focused = false;
                }
            },
            (ActivePane::CapConfig, _) => self.cap_config.handle_event(key)?,
        }
        Ok(())
    }
}

impl HotkeyProvider for ModuleView {
    fn hotkeys(&self) -> Vec<Hotkey> {
        let mut hk = vec![Hotkey::new("Tab", "Switch pane")];
        match self.active_pane {
            ActivePane::Code => hk.push(Hotkey::new("...", "Code editor")),
            ActivePane::CapConfig => hk.extend(self.cap_config.hotkeys()),
        }
        hk
    }
}