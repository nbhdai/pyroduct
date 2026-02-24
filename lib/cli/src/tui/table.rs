use std::collections::VecDeque;

use arrow::array::RecordBatch;
use pyroduct::pipeline::wasm::PyroLogs;
use pyroduct::value::arrow::Rowable;
use pyroduct::value::{PyroRow, PyroRowOwned, PyroValue};
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Row, Table};
use ratatui::Frame;

// =============================================================================
// Configuration
// =============================================================================

const MAX_CELL_WIDTH: usize = 40;
const NULL_DISPLAY: &str = "∅";

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

    /// Returns the resolved column names to display.
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

        let cols = self.display_columns();
        let col_count = cols.len();

        let header_cells: Vec<Cell> = std::iter::once(Cell::from("#").style(Style::default().fg(Color::DarkGray)))
            .chain(cols.iter().map(|name| {
                Cell::from(name.clone()).style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
            }))
            .collect();
        let header = Row::new(header_cells).height(1);

        let rows: Vec<Row> = self
            .buffer
            .iter()
            .enumerate()
            .map(|(buf_idx, row)| {
                let abs_idx = self.offset + buf_idx;
                let is_selected = abs_idx == self.selected;

                let idx_cell = Cell::from(format!("{}", abs_idx))
                    .style(Style::default().fg(Color::DarkGray));

                let data_cells = cols.iter().map(|col_name| {
                    let text = format_cell(row, col_name);
                    Cell::from(text)
                });

                let row_style = if is_selected {
                    Style::default()
                        .bg(Color::Rgb(40, 40, 60))
                        .add_modifier(Modifier::BOLD)
                } else if abs_idx % 2 == 0 {
                    Style::default()
                } else {
                    Style::default().bg(Color::Rgb(20, 20, 25))
                };

                Row::new(std::iter::once(idx_cell).chain(data_cells))
                    .style(row_style)
                    .height(1)
            })
            .collect();

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

fn format_cell(row: &PyroRow<'_>, col: &str) -> String {
    match row.get(col) {
        None | Some(PyroValue::Null) => NULL_DISPLAY.to_string(),
        Some(val) => {
            let s = format!("{}", val);
            if s.len() > MAX_CELL_WIDTH {
                let mut truncated = s[..MAX_CELL_WIDTH - 1].to_string();
                truncated.push('…');
                truncated
            } else {
                s
            }
        }
    }
}

fn digit_count(n: usize) -> usize {
    if n == 0 {
        return 1;
    }
    ((n as f64).log10().floor() as usize) + 1
}