use std::{
    fs,
    io::stdout,
    path::{Path, PathBuf},
    time::Duration,
    sync::Arc,
};

use arrow::array::RecordBatch;
use arrow::datatypes::Schema;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use pyroduct::pipeline::wasm_execute::PipelineExecution;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame, Terminal,
};
use ratatui_code_editor::{editor::Editor, theme::vesper};

pub mod nav;
pub mod logs;
pub mod wasm;
pub mod table;
pub mod output;
pub mod cap_config;

pub struct ModuleStep {
    pub name: String,
    pub path: PathBuf,
    pub source_code: String,
}

pub struct PipelineState {
    pub yaml_path: PathBuf,
    pub steps: Vec<ModuleStep>,
    pub input: RecordBatch,
    pub execution: Vec<PipelineExecution>,
}

pub enum ViewState {
    Code(wasm::CodeState),
    InputTable(table::TableView),
    OutputTable(output::OutputView),
}

pub struct App {
    pub pipeline: PipelineState,
    pub view: ViewState,
    pub status_msg: String,
    pub quit: bool,
}

impl App {
    pub fn load(yaml_path: &Path) -> Result<Self> {
        let config = crate::run::load_config(yaml_path)?;

        let mut steps = Vec::new();
        for module_name in &config.pipeline {
            let mod_conf = config
                .modules
                .get(module_name)
                .ok_or_else(|| anyhow::anyhow!("Module '{}' not in [modules]", module_name))?;

            let path = mod_conf.path.clone();
            let src_path = path.join("src/lib.rs");

            let source_code = fs::read_to_string(&src_path)
                .unwrap_or_else(|_| "// No source found\n".to_string());

            steps.push(ModuleStep {
                name: module_name.clone(),
                path,
                source_code,
            });
        }

        let initial_code = steps.first().map(|s| s.source_code.clone()).unwrap_or_default();
        let empty_batch = RecordBatch::new_empty(Arc::new(Schema::empty()));

        Ok(App {
            pipeline: PipelineState {
                yaml_path: yaml_path.to_path_buf(),
                steps,
                input: empty_batch,
                execution: Vec::new(),
            },
            view: ViewState::Code(wasm::CodeState {
                editing: false,
                area: Rect::default(),
                editor: Editor::new("rust", &initial_code, vesper()),
                selected_step: 0,
            }),
            status_msg: "i/Enter: focus │ ↑/↓: nav │ ^S: save │ ^R: run │ ^Q: quit".into(),
            quit: false,
        })
    }

    pub fn save(&mut self) {
        if let ViewState::Code(code_state) = &mut self.view {
            let step_idx = code_state.selected_step;
            self.pipeline.steps[step_idx].source_code = code_state.editor.get_content();
            
            let step = &self.pipeline.steps[step_idx];
            let path = step.path.join("src/lib.rs");
            
            match fs::write(&path, &step.source_code) {
                Ok(_) => {
                    self.status_msg = format!("Saved {}", path.display());
                }
                Err(e) => {
                    self.status_msg = format!("Save failed: {}", e);
                }
            }
        } else {
            self.status_msg = "Nothing to save here.".into();
        }
    }

    pub fn run_pipeline(&mut self) {
        self.save();
        self.status_msg = "Executing run... (Check console/logs)".into();

    }

    pub fn current_nav_index(&self) -> usize {
        match &self.view {
            ViewState::InputTable(_) => 0,
            ViewState::Code(c) => (c.selected_step * 2) + 1,
            ViewState::OutputTable(_) => self.pipeline.steps.len() + 1,
        }
    }

    pub fn nav_to(&mut self, new_idx: usize) {
        let max_idx = self.pipeline.steps.len() + 1;
        if new_idx > max_idx { return; }

        // Sync old code text to domain before navigating away
        if let ViewState::Code(c) = &self.view {
            self.pipeline.steps[c.selected_step].source_code = c.editor.get_content();
        }

        // Hydrate the new view from the domain state
        if new_idx == 0 {
            self.view = ViewState::InputTable(table::TableView::new(self.pipeline.input.clone()));
        } else {
            let stage_idx = (new_idx - 1)/2;
            let code_idx = new_idx % 2 == 1;
            if code_idx {
                let code = &self.pipeline.steps[stage_idx].source_code;
                self.view = ViewState::Code(wasm::CodeState {
                    editing: false,
                    area: ratatui::layout::Rect::default(),
                    editor: ratatui_code_editor::editor::Editor::new("rust", code, ratatui_code_editor::theme::vesper()),
                    selected_step: stage_idx,
                });
            } else {
                self.view = ViewState::OutputTable(output::OutputView::new(self.pipeline.execution.clone(), stage_idx).unwrap());
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(1)])
        .split(f.area());

    let horizontal_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(25)])
        .split(vertical_chunks[0]);

    let main_area = horizontal_chunks[0];
    let nav_area = horizontal_chunks[1];

    match &mut app.view {
        ViewState::Code(code_state) => {
            wasm::render(f, &app.pipeline, code_state, main_area);
        }
        ViewState::InputTable(table_view) => {
            table_view.render(f, main_area, "Input Table");
        }
        ViewState::OutputTable(table_view) => {
            table_view.render(f, main_area, "Output Table");
        }
    }

    nav::render(f, app, nav_area);
    render_status(f, app, vertical_chunks[1]);
}

fn render_status(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = match &app.view {
        ViewState::Code(s) => s.editing,
        ViewState::InputTable(s) => s.focused,
        ViewState::OutputTable(s) => s.focused,
    };

    let mode = if is_focused {
        Span::styled(" FOCUS ", Style::default().fg(Color::Black).bg(Color::Green))
    } else {
        Span::styled("  NAV  ", Style::default().fg(Color::Black).bg(Color::Blue))
    };

    let line = Line::from(vec![
        mode,
        Span::raw(" │ "),
        Span::styled(&app.status_msg, Style::default().fg(Color::DarkGray)),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

fn handle_event(app: &mut App) -> Result<()> {
    if !event::poll(Duration::from_millis(50))? {
        return Ok(());
    }
    let ev = event::read()?;
    let Event::Key(key) = ev else { return Ok(()) };

    // Global Hotkeys
    match (key.modifiers, key.code) {
        (KeyModifiers::CONTROL, KeyCode::Char('c'))
        | (KeyModifiers::CONTROL, KeyCode::Char('q')) => {
            app.quit = true;
            return Ok(());
        }
        (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
            app.save();
            return Ok(());
        }
        (KeyModifiers::CONTROL, KeyCode::Char('r')) => {
            app.run_pipeline();
            return Ok(());
        }
        _ => {}
    }

    let is_focused = match &app.view {
        ViewState::Code(s) => s.editing,
        ViewState::InputTable(s) => s.focused,
        ViewState::OutputTable(s) => s.focused,
    };

    if !is_focused {
        match key.code {
            KeyCode::Enter | KeyCode::Char('i') => {
                match &mut app.view {
                    ViewState::Code(s) => s.editing = true,
                    ViewState::InputTable(s) => s.focused = true,
                    ViewState::OutputTable(s) => s.focused = true,
                }
                app.status_msg = "Focused ─ Esc to return".into();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let idx = app.current_nav_index();
                if idx > 0 { app.nav_to(idx - 1); }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let idx = app.current_nav_index();
                app.nav_to(idx + 1);
            }
            _ => {}
        }
    } else {
        if key.code == KeyCode::Esc {
            match &mut app.view {
                ViewState::Code(s) => s.editing = false,
                ViewState::InputTable(s) => s.focused = false,
                ViewState::OutputTable(s) => s.focused = false,
            }
            app.status_msg = "Navigation mode".into();
        } else {
            // Forward event to currently active widget
            match &mut app.view {
                ViewState::Code(s) => {
                    wasm::handle_event(s, key)?;
                }
                ViewState::InputTable(t) => t.handle_event(key),
                ViewState::OutputTable(t) => t.handle_event(key),
            }
        }
    }

    Ok(())
}

pub fn run_tui(yaml_path: &Path) -> Result<()> {
    if !yaml_path.exists() {
        anyhow::bail!("File not found: {}", yaml_path.display());
    }

    let mut app = App::load(yaml_path)?;

    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|f| ui(f, &mut app))?;
        handle_event(&mut app)?;
        
        if app.quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}