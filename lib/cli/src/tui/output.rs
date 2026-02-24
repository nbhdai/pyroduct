use std::sync::Arc;
use arrow::array::RecordBatch;
use arrow::datatypes::Schema;
use crossterm::event::{KeyCode, KeyEvent};
use pyroduct::pipeline::{wasm::PyroLogs, wasm_execute::extract_upto_batch};
use pyroduct::pipeline::wasm_execute::PipelineExecution;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use super::{logs::LogsView, table::TableView};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePane {
    Table,
    Logs,
}

pub struct OutputView {
    pub table: TableView,
    pub logs: LogsView,
    pub executions: Vec<PipelineExecution>,
    pub step_index: usize,
    pub active_pane: ActivePane,
    pub focused: bool,
    current_row: usize,
}


// Add at the top of output.rs, with the other use statements:
use super::keys::{Hotkey, HotkeyProvider};

// Add at the bottom of output.rs:
impl HotkeyProvider for OutputView {
    fn hotkeys(&self) -> Vec<Hotkey> {
        let mut hk = vec![Hotkey::new("Tab", "Switch pane")];
        match self.active_pane {
            ActivePane::Table => hk.extend(self.table.hotkeys()),
            ActivePane::Logs => hk.extend(self.logs.hotkeys()),
        }
        hk
    }
}


impl OutputView {
    pub fn new(executions: Vec<PipelineExecution>, step_index: usize) -> anyhow::Result<Self> {
        let batch = extract_upto_batch(&executions, step_index)?.unwrap_or_else(|| RecordBatch::new_empty(Arc::new(Schema::empty())));
        let table = TableView::new(batch);
        let logs = LogsView::default();

        Ok(Self {
            table,
            logs,
            executions,
            step_index,
            active_pane: ActivePane::Table,
            focused: false,
            current_row: 0,
        })
    }

    fn extract_logs(
        &self, 
        row_index: usize,
    ) -> PyroLogs {
        if let Some(exec) = self.executions.get(row_index) {
            // Fetch logs for successful steps
            if let Some(step) = exec.steps.get(self.step_index) {
                return PyroLogs {
                    module_logs: step.logs.module_logs.clone(),
                    capability_logs: step.logs.capability_logs.clone(),
                };
            } 
            // Fetch logs for failed step (if it failed exactly on this step)
            else if let Some(fail) = &exec.failure {
                if exec.steps.len() == self.step_index {
                    return PyroLogs {
                        module_logs: fail.logs.module_logs.clone(),
                        capability_logs: fail.logs.capability_logs.clone(),
                    };
                }
            }
        }
        PyroLogs::empty()
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect, title: &str) {
        // Keep logs in sync with table selection
        if self.table.selected() != self.current_row {
            self.current_row = self.table.selected();
            let new_logs = self.extract_logs(self.current_row);
            self.logs = LogsView::new(new_logs);
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(area);

        let orig_table_focus = self.table.focused;
        self.table.focused = self.focused && self.active_pane == ActivePane::Table;
        self.table.render(f, chunks[0], title);
        self.table.focused = orig_table_focus;

        let logs_focused = self.focused && self.active_pane == ActivePane::Logs;
        self.logs.render(f, chunks[1], "Logs", logs_focused);
    }

    pub fn handle_event(&mut self, key: KeyEvent) {
        // Toggle focus with Tab
        if key.code == KeyCode::Tab {
            self.active_pane = match self.active_pane {
                ActivePane::Table => ActivePane::Logs,
                ActivePane::Logs => ActivePane::Table,
            };
            return;
        }

        match (self.active_pane, key.code) {
            (_, KeyCode::Esc) => self.focused = false,
            (ActivePane::Table, KeyCode::Up | KeyCode::Char('k')) => self.table.scroll_up(1),
            (ActivePane::Table, KeyCode::Down | KeyCode::Char('j')) => self.table.scroll_up(1),
            (ActivePane::Table, KeyCode::PageUp) => self.table.page_up(),
            (ActivePane::Table, KeyCode::PageDown) => self.table.page_down(),
            (ActivePane::Table, KeyCode::Home) => self.table.home(),
            (ActivePane::Table, KeyCode::End) => self.table.end(),
            (ActivePane::Table, _) => {},
            (ActivePane::Logs, _) => self.logs.handle_event(key),
        }
    }
}