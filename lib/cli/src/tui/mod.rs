// lib/cli/src/tui/mod.rs
use std::{
    fs,
    io::stdout,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use arrow::array::RecordBatch;
use arrow::datatypes::Schema;

use crate::run::load_config;
use anyhow::Result;
use pyro_artifacts::cache::CacheManager;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use pyroduct::pipeline::{PipelineConfig, PipelineFactory};
use pyroduct::pipeline::{PipelinePool, wasm_execute::PipelineExecution};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    prelude::*,
    style::Style,
};
use ratatui_code_editor::{editor::Editor, theme::vesper};

pub mod cap_config;
pub mod keys;
pub mod logs;
pub mod module;
pub mod nav;
pub mod output;
pub mod table;
pub mod wasm;

use keys::{Hotkey, HotkeyProvider};

pub struct PipelineState {
    pub yaml_path: PathBuf,
    pub tui: PipelineConfig,
    pub input: RecordBatch,
    pub execution: Vec<PipelineExecution>,
}

pub enum ViewState {
    Module(module::ModuleView),
    InputTable(table::TableView),
    OutputTable(output::OutputView),
}

pub struct App {
    pub cache: CacheManager,
    pub pipeline: PipelineState,
    pub view: ViewState,
    pub status_msg: String,
    pub quit: bool,
}

impl App {
    pub async fn load(yaml_path: &Path, input_path: &Path) -> Result<Self> {
        let cache = CacheManager::from_env().await?;
        let mut pipeline_config = load_config(yaml_path).await?;
        pipeline_config.load_sources(&cache).await?;

        let input_batch = Self::load_input(input_path).await?;

        // Get initial state from the first module if it exists
        let (initial_code, initial_caps) =
            if let Some((_, module)) = pipeline_config.pipeline.get_index(0) {
                let code = match &module.module {
                    pyroduct::module::Module::Source(s) => s.source.clone(),
                    _ => String::new(),
                };
                let caps: Vec<(String, String)> = module
                    .configurations
                    .iter()
                    .map(|(k, v)| (k.clone(), serde_json::to_string(v).unwrap_or_default()))
                    .collect();
                (code, caps)
            } else {
                (String::new(), Vec::new())
            };

        let code_state = wasm::CodeState {
            editing: false,
            area: Rect::default(),
            editor: Editor::new("rust", &initial_code, vesper()),
            selected_step: 0,
        };
        let cap_state = cap_config::CapConfigState::new(initial_caps);

        Ok(App {
            cache,
            pipeline: PipelineState {
                yaml_path: yaml_path.to_path_buf(),
                tui: pipeline_config,
                input: input_batch,
                execution: Vec::new(),
            },
            view: ViewState::Module(module::ModuleView::new(code_state, cap_state)),
            status_msg: String::new(),
            quit: false,
        })
    }

    async fn load_input(input_path: &Path) -> Result<RecordBatch> {
        if !input_path.exists() {
            anyhow::bail!("Input file does not exist: {}", input_path.display());
        }
        let bytes = fs::read(input_path)?;
        let filename = input_path.file_name().unwrap_or_default().to_string_lossy();
        let batches = pyro_arrow_file::parse_data_to_batch(bytes, &filename).await?;
        if batches.is_empty() {
            Ok(RecordBatch::new_empty(Arc::new(Schema::empty())))
        } else {
            Ok(batches[0].clone().to_batch())
        }
    }

    pub async fn save(&mut self) {
        if let ViewState::Module(mv) = &mut self.view {
            let step_idx = mv.selected_step();
            let Some((name, module)) = self.pipeline.tui.pipeline.get_index_mut(step_idx) else {
                self.status_msg = "Error: selected module not found".into();
                return;
            };

            if let pyroduct::module::Module::Source(s) = &mut module.module {
                s.source = mv.code.editor.get_content();
            }

            // We need a way to compile from ModuleConfig directly or similar.
            // For now, let's keep the logic of extracting bits.
            match &module.module {
                pyroduct::module::Module::Source(source) => {
                    match self.cache.compile(&source).await {
                        Ok(_) => {
                            // module.artifact = Some(artifact); // ModuleConfig doesn't have artifact field now
                            self.status_msg = format!("Compiled {}", name);
                        }
                        Err(e) => {
                            self.status_msg = format!("Compilation failed: {}", e);
                        }
                    }
                }
                pyroduct::module::Module::Hash(_) | pyroduct::module::Module::Path(_) => {
                    unreachable!()
                }
            };

            // Persistence
            let tui_path = self.pipeline.yaml_path.with_extension("tui.json");
            let json = serde_json::to_string_pretty(&self.pipeline.tui).unwrap_or_default();
            if let Err(e) = std::fs::write(&tui_path, json) {
                self.status_msg = format!("Failed to save TUI state: {}", e);
            }
        } else {
            self.status_msg = "Nothing to save here.".into();
        }
    }

    async fn run_pipeline_inner(&mut self) -> Result<()> {
        let mut factory = PipelineFactory::load(&self.pipeline.tui).await?;
        let pipeline = factory.build().await?;
        let pool = PipelinePool::new(vec![pipeline]);

        let (successes, failures) = pool.process_batch(&self.pipeline.input).await?;

        let mut all_executions = successes;
        all_executions.extend(failures);
        all_executions.sort_by_key(|e| e.row_index);
        self.pipeline.execution = all_executions;

        // If currently viewing the output table, refresh it seamlessly
        if let ViewState::OutputTable(ov) = &mut self.view {
            let stage_idx = ov.step_index;
            if let Ok(new_ov) =
                crate::tui::output::OutputView::new(self.pipeline.execution.clone(), stage_idx)
            {
                *ov = new_ov;
            }
        }
        Ok(())
    }

    pub async fn run_pipeline(&mut self) {
        self.save().await;
        self.status_msg = "Executing run...".into();

        if let Err(e) = self.run_pipeline_inner().await {
            self.status_msg = format!("Run failed: {}", e);
        } else {
            self.status_msg = "Run complete!".into();
        }
    }

    pub fn current_nav_index(&self) -> usize {
        match &self.view {
            ViewState::InputTable(_) => 0,
            ViewState::Module(mv) => (mv.selected_step() * 2) + 1,
            ViewState::OutputTable(_) => self.pipeline.tui.pipeline.len() + 1,
        }
    }

    pub fn nav_to(&mut self, new_idx: usize) {
        let max_idx = self.pipeline.tui.pipeline.len() + 1;
        if new_idx > max_idx {
            return;
        }

        // Sync old code text to domain before navigating away
        if let ViewState::Module(mv) = &self.view {
            let step_idx = mv.selected_step();
            if let Some((_, module)) = self.pipeline.tui.pipeline.get_index_mut(step_idx) {
                if let pyroduct::module::Module::Source(s) = &mut module.module {
                    s.source = mv.code.editor.get_content();
                }
            }
        }

        // Hydrate the new view from the domain state
        if new_idx == 0 {
            self.view = ViewState::InputTable(table::TableView::new(self.pipeline.input.clone()));
        } else {
            let stage_idx = (new_idx - 1) / 2;
            let is_code = new_idx % 2 == 1;
            if is_code {
                let Some((_, module)) = self.pipeline.tui.pipeline.get_index(stage_idx) else {
                    return;
                };
                let source = match &module.module {
                    pyroduct::module::Module::Source(s) => s.source.clone(),
                    _ => String::new(),
                };
                let code_state = wasm::CodeState {
                    editing: false,
                    area: Rect::default(),
                    editor: Editor::new("rust", &source, vesper()),
                    selected_step: stage_idx,
                };

                let initial_caps: Vec<(String, String)> = module
                    .configurations
                    .iter()
                    .map(|(k, v)| (k.clone(), serde_json::to_string(v).unwrap_or_default()))
                    .collect();

                let cap_state = cap_config::CapConfigState::new(initial_caps);
                self.view = ViewState::Module(module::ModuleView::new(code_state, cap_state));
            } else {
                self.view = ViewState::OutputTable(
                    output::OutputView::new(self.pipeline.execution.clone(), stage_idx + 1)
                        .unwrap(),
                );
            }
        }
    }
}

fn ui(f: &mut Frame<'_>, app: &mut App) {
    let status_height = if app.status_msg.is_empty() { 1 } else { 2 };

    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(status_height)])
        .split(f.area());

    let horizontal_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(25)])
        .split(vertical_chunks[0]);

    let main_area = horizontal_chunks[0];
    let nav_area = horizontal_chunks[1];

    match &mut app.view {
        ViewState::Module(mv) => {
            mv.render(f, &app.pipeline, main_area);
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

impl HotkeyProvider for App {
    fn hotkeys(&self) -> Vec<Hotkey> {
        let mut hk = vec![
            Hotkey::new("^Q", "Quit"),
            Hotkey::new("^S", "Save"),
            Hotkey::new("^R", "Run"),
        ];

        let is_focused = match &self.view {
            ViewState::Module(mv) => mv.focused,
            ViewState::InputTable(s) => s.focused,
            ViewState::OutputTable(s) => s.focused,
        };

        if !is_focused {
            hk.push(Hotkey::new("↑/↓", "Navigate"));
            hk.push(Hotkey::new("Enter", "Focus"));
        } else {
            hk.push(Hotkey::new("Esc", "Unfocus"));
            match &self.view {
                ViewState::Module(mv) => hk.extend(mv.hotkeys()),
                ViewState::InputTable(tv) => hk.extend(tv.hotkeys()),
                ViewState::OutputTable(ov) => hk.extend(ov.hotkeys()),
            }
        }

        hk
    }
}

fn render_status(f: &mut Frame, app: &App, area: Rect) {
    if !app.status_msg.is_empty() {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(area);

        let style = if app.status_msg.contains("failed") || app.status_msg.contains("Failed") {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::Green)
        };
        let status =
            ratatui::widgets::Paragraph::new(ratatui::text::Span::styled(&app.status_msg, style));
        f.render_widget(status, chunks[0]);

        let hk = app.hotkeys();
        keys::render(f, &hk, chunks[1]);
    } else {
        let hk = app.hotkeys();
        keys::render(f, &hk, area);
    }
}

async fn handle_event(app: &mut App) -> Result<()> {
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
            app.save().await;
            return Ok(());
        }
        (KeyModifiers::CONTROL, KeyCode::Char('r')) => {
            app.run_pipeline().await;
            return Ok(());
        }
        _ => {}
    }

    let is_focused = match &app.view {
        ViewState::Module(mv) => mv.focused,
        ViewState::InputTable(s) => s.focused,
        ViewState::OutputTable(s) => s.focused,
    };

    if !is_focused {
        match key.code {
            KeyCode::Enter | KeyCode::Char('i') => {
                match &mut app.view {
                    ViewState::Module(mv) => {
                        mv.focused = true;
                        // Only code pane enters editing immediately.
                        // Cap config starts in tab-navigation mode.
                        if mv.active_pane == module::ActivePane::Code {
                            mv.code.editing = true;
                        }

                        // Proactively refresh config if that tab is visible
                        if mv.bottom_tab == module::BottomTab::Config {
                            mv.cap_config.refresh_available_caps(&app.cache).await;
                            if mv.cap_config.selected_tab < mv.cap_config.editors.len() {
                                let name =
                                    mv.cap_config.editors[mv.cap_config.selected_tab].0.clone();
                                mv.cap_config.load_interface(&app.cache, &name).await;
                            }
                        }
                    }
                    ViewState::InputTable(s) => s.focused = true,
                    ViewState::OutputTable(s) => s.focused = true,
                }
                app.status_msg = String::new();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let idx = app.current_nav_index();
                if idx > 0 {
                    app.nav_to(idx - 1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let idx = app.current_nav_index();
                app.nav_to(idx + 1);
            }
            _ => {}
        }
    } else {
        // Forward event to currently active widget
        match &mut app.view {
            ViewState::Module(mv) => {
                mv.handle_event(key, &app.cache).await?;
            }
            ViewState::InputTable(t) => t.handle_event(key),
            ViewState::OutputTable(t) => t.handle_event(key),
        }
    }

    Ok(())
}

pub async fn run_tui(yaml_path: &Path, input_path: &Path) -> Result<()> {
    if !yaml_path.exists() {
        anyhow::bail!("File not found: {}", yaml_path.display());
    }

    let mut app = App::load(yaml_path, input_path).await?;

    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|f| ui(f, &mut app))?;
        handle_event(&mut app).await?;

        if app.quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}
