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

        // Only mark cap_config as "active" (highlighted border) when it's the
        // focused pane — but do NOT override its internal `editing` flag.
        // The cap_config pane manages its own editing state: it starts in
        // tab-navigation mode (editing=false) so the user can browse tabs and
        // reach the "+" tab, and only enters editing mode on explicit Enter.
        let is_active_pane = self.focused && self.active_pane == ActivePane::CapConfig;
        self.cap_config.active = is_active_pane;
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
                if self.code.editing {
                    self.code.editing = false;
                } else {
                    self.focused = false;
                }
            },
            (ActivePane::Code, _) => wasm::handle_event(&mut self.code, key)?,
            (ActivePane::CapConfig, KeyCode::Esc) => {
                if self.cap_config.editing {
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