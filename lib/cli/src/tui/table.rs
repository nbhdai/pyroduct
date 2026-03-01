use std::collections::VecDeque;

use arrow::array::RecordBatch;
use arrow::datatypes::DataType;
use crossterm::event::KeyCode;
use pyroduct::value::arrow::Rowable;
use pyroduct::value::{PyroRow, PyroRowOwned, PyroValue};
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Row, Table};
use ratatui::Frame;

use super::keys::{Hotkey, HotkeyProvider};

// =============================================================================
// Configuration
// =============================================================================

const MAX_CELL_WIDTH: usize = 40;
const NULL_DISPLAY: &str = "∅";

// =============================================================================
// FlatColumn — a resolved display column with a dotted path
// =============================================================================

/// Describes a single flattened column for display.
/// For a top-level scalar like `name`, path is `["name"]`.
/// For a struct field like `address.city`, path is `["address", "city"]`.
/// For a list-of-structs like `input` (List<Struct{role, content}>),
/// path is `["input", "role"]` and `list_parent` is `Some("input")`.
#[derive(Debug, Clone)]
struct FlatColumn {
    /// The dot-joined display header, e.g. "input.role"
    header: String,
    /// Path segments for value lookup, e.g. ["input", "role"]
    path: Vec<String>,
    /// If this column was expanded from a List<Struct>, the top-level field name.
    /// All FlatColumns sharing the same `list_parent` expand together.
    list_parent: Option<String>,
}

// =============================================================================
// TableView
// =============================================================================

/// A scrollable table view over a RecordBatch, backed by a VecDeque of
/// pre-materialized PyroRows for the visible window.
pub struct TableView {
    /// The source data.
    batch: RecordBatch,
    /// Which columns to display (by name). If empty, show all.
    columns: Vec<String>,
    /// Index of the first visible row in the batch.
    offset: usize,
    /// How many rows fit in the visible area (updated on each render).
    page_size: usize,
    /// The currently selected row (absolute index into the batch).
    selected: usize,
    /// Pre-materialized rows for the visible window.
    buffer: VecDeque<PyroRowOwned>,
    /// Tracks which batch range is currently buffered: (start, end exclusive).
    buffered_range: (usize, usize),

    pub focused: bool,
}


impl TableView {
    pub fn new(batch: RecordBatch) -> Self {
        Self {
            batch,
            columns: Vec::new(),
            offset: 0,
            page_size: 20,
            selected: 0,
            buffer: VecDeque::new(),
            buffered_range: (0, 0),
            focused: false,
        }
    }

    pub fn handle_event(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            KeyCode::Esc => self.focused = false,
            KeyCode::Up | KeyCode::Char('k') => self.scroll_up(1),
            KeyCode::Down | KeyCode::Char('j') => self.scroll_down(1),
            KeyCode::PageUp => self.page_up(),
            KeyCode::PageDown => self.page_down(),
            KeyCode::Home => self.home(),
            KeyCode::End => self.end(),
            _ => {}
        }
    }

    pub fn with_columns(mut self, columns: Vec<String>) -> Self {
        self.columns = columns;
        self
    }

    // -------------------------------------------------------------------------
    // Data access
    // -------------------------------------------------------------------------

    pub fn total_rows(&self) -> usize {
        self.batch.num_rows()
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Replace the backing batch (e.g. when new data arrives).
    pub fn set_batch(&mut self, batch: RecordBatch) {
        self.batch = batch;
        self.selected = self.selected.min(self.total_rows().saturating_sub(1));
        self.offset = self.offset.min(self.total_rows().saturating_sub(1));
        self.invalidate_buffer();
    }

    /// Returns the resolved column names to display (top-level only, for compatibility).
    fn display_columns(&self) -> Vec<String> {
        if self.columns.is_empty() {
            self.batch
                .schema()
                .fields()
                .iter()
                .map(|f| f.name().clone())
                .collect()
        } else {
            self.columns
                .iter()
                .filter(|c| self.batch.schema().column_with_name(c).is_some())
                .map(|c| c.clone())
                .collect()
        }
    }

    /// Build flattened column descriptors from the Arrow schema.
    ///
    /// - Scalar fields → single FlatColumn with the field name
    /// - Struct fields → one FlatColumn per child, header = "parent.child"
    /// - List<Struct> fields → one FlatColumn per struct child, header = "parent.child",
    ///   with `list_parent` set so the renderer knows to expand rows
    /// - Other list/complex fields → single FlatColumn with the field name (rendered inline)
    fn flat_columns(&self) -> Vec<FlatColumn> {
        let schema = self.batch.schema();
        let top_fields: Vec<String> = self.display_columns();
        let mut out = Vec::new();

        for name in &top_fields {
            let Some(field) = schema.field_with_name(name).ok() else {
                continue;
            };
            match field.data_type() {
                // Struct → flatten children
                DataType::Struct(children) => {
                    for child in children {
                        out.push(FlatColumn {
                            header: format!("{}.{}", name, child.name()),
                            path: vec![name.clone(), child.name().clone()],
                            list_parent: None,
                        });
                    }
                }
                // List<Struct> → flatten struct children, mark as list-expandable
                DataType::List(inner) | DataType::LargeList(inner) => {
                    if let DataType::Struct(children) = inner.data_type() {
                        for child in children {
                            out.push(FlatColumn {
                                header: format!("{}.{}", name, child.name()),
                                path: vec![name.clone(), child.name().clone()],
                                list_parent: Some(name.clone()),
                            });
                        }
                    } else {
                        // List of primitives/strings → show inline
                        out.push(FlatColumn {
                            header: name.clone(),
                            path: vec![name.clone()],
                            list_parent: None,
                        });
                    }
                }
                // Scalar / everything else
                _ => {
                    out.push(FlatColumn {
                        header: name.clone(),
                        path: vec![name.clone()],
                        list_parent: None,
                    });
                }
            }
        }
        out
    }

    // -------------------------------------------------------------------------
    // Buffer management
    // -------------------------------------------------------------------------

    fn invalidate_buffer(&mut self) {
        self.buffer.clear();
        self.buffered_range = (0, 0);
    }

    /// Ensures `buffer` contains materialized rows for [start..end).
    fn ensure_buffered(&mut self, start: usize, end: usize) {
        let end = end.min(self.total_rows());
        if start >= end {
            self.buffer.clear();
            self.buffered_range = (start, start);
            return;
        }

        if self.buffered_range == (start, end) && self.buffer.len() == end - start {
            return;
        }

        let (buf_start, buf_end) = self.buffered_range;

        if start >= buf_start && start < buf_end && end <= buf_end {
            let trim_front = start - buf_start;
            let trim_back = buf_end - end;
            for _ in 0..trim_front {
                self.buffer.pop_front();
            }
            for _ in 0..trim_back {
                self.buffer.pop_back();
            }
            self.buffered_range = (start, end);
            return;
        }

        if start >= buf_start && start <= buf_end && end > buf_end {
            let trim_front = start - buf_start;
            for _ in 0..trim_front {
                self.buffer.pop_front();
            }
            for i in buf_end..end {
                if let Ok(row) = self.batch.row(i) {
                    self.buffer.push_back(row.into_owned());
                }
            }
            self.buffered_range = (start, end);
            return;
        }

        if start < buf_start && end >= buf_start && end <= buf_end {
            let trim_back = buf_end - end;
            for _ in 0..trim_back {
                self.buffer.pop_back();
            }
            for i in (start..buf_start).rev() {
                if let Ok(row) = self.batch.row(i) {
                    self.buffer.push_front(row.into_owned());
                }
            }
            self.buffered_range = (start, end);
            return;
        }

        self.buffer.clear();
        for i in start..end {
            if let Ok(row) = self.batch.row(i) {
                self.buffer.push_back(row.into_owned());
            }
        }
        self.buffered_range = (start, end);
    }

    // -------------------------------------------------------------------------
    // Navigation
    // -------------------------------------------------------------------------

    pub fn scroll_down(&mut self, n: usize) {
        let max = self.total_rows().saturating_sub(1);
        self.selected = (self.selected + n).min(max);
        if self.selected >= self.offset + self.page_size {
            self.offset = self.selected - self.page_size + 1;
        }
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.selected = self.selected.saturating_sub(n);
        if self.selected < self.offset {
            self.offset = self.selected;
        }
    }

    pub fn page_down(&mut self) {
        self.scroll_down(self.page_size);
    }

    pub fn page_up(&mut self) {
        self.scroll_up(self.page_size);
    }

    pub fn home(&mut self) {
        self.selected = 0;
        self.offset = 0;
    }

    pub fn end(&mut self) {
        let max = self.total_rows().saturating_sub(1);
        self.selected = max;
        self.offset = max.saturating_sub(self.page_size.saturating_sub(1));
    }

    // -------------------------------------------------------------------------
    // Rendering
    // -------------------------------------------------------------------------

    pub fn render(&mut self, f: &mut Frame, area: Rect, title: &str) {
        let available_rows = area.height.saturating_sub(4) as usize;
        self.page_size = available_rows.max(1);

        let border_color = if self.focused { Color::Green } else { Color::Cyan };

        let total = self.total_rows();
        if total == 0 {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} (Empty) ", title))
                .border_style(Style::default().fg(border_color));
            f.render_widget(block, area);
            return;
        }

        self.selected = self.selected.min(total - 1);
        self.offset = self.offset.min(total.saturating_sub(1));
        if self.selected >= self.offset + self.page_size {
            self.offset = self.selected - self.page_size + 1;
        }

        let view_end = (self.offset + self.page_size).min(total);
        self.ensure_buffered(self.offset, view_end);

        let flat_cols = self.flat_columns();
        let col_count = flat_cols.len();

        // Collect unique list parents to know which fields need expansion
        let list_parents: Vec<String> = {
            let mut seen = Vec::new();
            for fc in &flat_cols {
                if let Some(ref lp) = fc.list_parent {
                    if !seen.contains(lp) {
                        seen.push(lp.clone());
                    }
                }
            }
            seen
        };
        let has_list_expansion = !list_parents.is_empty();

        // Build header
        let header_cells: Vec<Cell> = std::iter::once(
            Cell::from("#").style(Style::default().fg(Color::DarkGray)),
        )
        .chain(flat_cols.iter().map(|fc| {
            Cell::from(fc.header.clone()).style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
        }))
        .collect();
        let header = Row::new(header_cells).height(1);

        // Build rows — potentially multiple visual rows per data row
        let mut rows: Vec<Row> = Vec::new();
        for (buf_idx, row) in self.buffer.iter().enumerate() {
            let abs_idx = self.offset + buf_idx;
            let is_selected = abs_idx == self.selected;

            let row_style = if is_selected {
                Style::default()
                    .bg(Color::Rgb(40, 40, 60))
                    .add_modifier(Modifier::BOLD)
            } else if abs_idx % 2 == 0 {
                Style::default()
            } else {
                Style::default().bg(Color::Rgb(20, 20, 25))
            };

            if !has_list_expansion {
                // Simple case: no list expansion needed
                let idx_cell = Cell::from(format!("{}", abs_idx))
                    .style(Style::default().fg(Color::DarkGray));
                let data_cells = flat_cols.iter().map(|fc| {
                    let text = format_cell_path(row, &fc.path);
                    Cell::from(text)
                });
                rows.push(
                    Row::new(std::iter::once(idx_cell).chain(data_cells))
                        .style(row_style)
                        .height(1),
                );
            } else {
                // Determine how many visual rows this data row needs.
                // It's the max list length across all list_parent fields.
                let expansion_count = list_parents
                    .iter()
                    .map(|lp| match row.get(lp) {
                        Some(PyroValue::List(items)) => items.len().max(1),
                        _ => 1,
                    })
                    .max()
                    .unwrap_or(1);

                for sub_idx in 0..expansion_count {
                    let idx_cell = if sub_idx == 0 {
                        Cell::from(format!("{}", abs_idx))
                            .style(Style::default().fg(Color::DarkGray))
                    } else {
                        Cell::from("".to_string())
                            .style(Style::default().fg(Color::DarkGray))
                    };

                    let data_cells = flat_cols.iter().map(|fc| {
                        let text = if fc.list_parent.is_some() {
                            // This column is from a List<Struct>. Extract the sub_idx-th
                            // element from the list, then get the child field.
                            format_list_struct_cell(row, &fc.path, sub_idx)
                        } else if sub_idx == 0 {
                            // Non-list column: only show on first sub-row
                            format_cell_path(row, &fc.path)
                        } else {
                            String::new()
                        };
                        Cell::from(text)
                    });

                    rows.push(
                        Row::new(std::iter::once(idx_cell).chain(data_cells))
                            .style(row_style)
                            .height(1),
                    );
                }
            }
        }

        let idx_width = digit_count(total) + 1;
        let remaining = area.width.saturating_sub(idx_width as u16 + 2);
        let per_col = if col_count > 0 {
            (remaining as usize / col_count).min(MAX_CELL_WIDTH).max(6)
        } else {
            10
        };

        let mut constraints = vec![Constraint::Length(idx_width as u16)];
        for _ in 0..col_count {
            constraints.push(Constraint::Length(per_col as u16));
        }

        let status = format!(
            " Row {}/{} | Showing {}-{} ",
            self.selected + 1,
            total,
            self.offset + 1,
            view_end,
        );

        let table = Table::new(rows, &constraints)
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" {} ", title))
                    .title_bottom(Line::from(Span::styled(
                        status,
                        Style::default().fg(Color::DarkGray),
                    )))
                    .border_style(Style::default().fg(border_color)),
            )
            .row_highlight_style(
                Style::default()
                    .bg(Color::Rgb(40, 40, 60))
                    .add_modifier(Modifier::BOLD),
            );

        f.render_widget(table, area);
    }
}

// =============================================================================
// Helpers
// =============================================================================

/// Format a cell by following a path of keys into the row.
/// For a single-segment path like ["name"], this is equivalent to the old format_cell.
/// For multi-segment like ["address", "city"], it traverses into Groups.
fn format_cell_path(row: &PyroRow<'_>, path: &[String]) -> String {
    if path.is_empty() {
        return NULL_DISPLAY.to_string();
    }

    // Use get_deep which already handles nested traversal
    let path_refs: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
    match row.get_deep(&path_refs) {
        None | Some(PyroValue::Null) => NULL_DISPLAY.to_string(),
        Some(val) => truncate_display(val),
    }
}

/// For a List<Struct> column: path is [list_field, child_field].
/// We get the list at path[0], index into it at `list_idx`, then get path[1] from the struct.
fn format_list_struct_cell(row: &PyroRow<'_>, path: &[String], list_idx: usize) -> String {
    if path.len() < 2 {
        return NULL_DISPLAY.to_string();
    }

    let list_val = row.get(&path[0]);
    match list_val {
        Some(PyroValue::List(items)) => {
            if let Some(item) = items.get(list_idx) {
                match item {
                    PyroValue::Group(inner_row) => {
                        match inner_row.get(&path[1]) {
                            None | Some(PyroValue::Null) => NULL_DISPLAY.to_string(),
                            Some(val) => truncate_display(val),
                        }
                    }
                    // If the list element is not a struct, show it on the first sub-column only
                    other => {
                        if path[1] == path[0] {
                            truncate_display(other)
                        } else {
                            NULL_DISPLAY.to_string()
                        }
                    }
                }
            } else {
                String::new() // past the end of the list
            }
        }
        _ => NULL_DISPLAY.to_string(),
    }
}

fn truncate_display(val: &PyroValue<'_>) -> String {
    let s = format!("{}", val);
    if s.len() > MAX_CELL_WIDTH {
        let mut truncated = s[..MAX_CELL_WIDTH - 1].to_string();
        truncated.push('…');
        truncated
    } else {
        s
    }
}

fn digit_count(n: usize) -> usize {
    if n == 0 {
        return 1;
    }
    ((n as f64).log10().floor() as usize) + 1
}

impl HotkeyProvider for TableView {
    fn hotkeys(&self) -> Vec<Hotkey> {
        vec![
            Hotkey::new("↑/k ↓/j", "Scroll"),
            Hotkey::new("PgUp/Dn", "Page"),
            Hotkey::new("Home/End", "Jump"),
        ]
    }
}