// lib/cli/src/tui/mod.rs
use std::{
    collections::HashMap,
    fs,
    io::stdout,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use arrow::array::RecordBatch;
use arrow::datatypes::Schema;

use anyhow::{Context as _, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use pyro_artifacts::{build::Builder, cache::CacheManager, cargo::ResolvedCapability};
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

fn default_wal_capacity() -> usize {
    1000
}
fn default_success_retention() -> u64 {
    3600
}
fn default_error_retention() -> u64 {
    86400 * 7
}

/// A single pipeline configuration in the TUI.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct SourcePipeline {
    pub name: String,
    pub source: String,
    pub compile_error: Option<String>,
    #[serde(default)]
    pub capabilities: HashMap<ResolvedCapability, Option<serde_json::Value>>,
    #[serde(default = "default_wal_capacity")]
    pub wal_capacity: usize,
    #[serde(default = "default_success_retention")]
    pub success_log_retention_secs: u64,
    #[serde(default = "default_error_retention")]
    pub error_log_retention_secs: u64,
}

pub struct PipelineState {
    pub yaml_path: PathBuf,
    pub pipelines: Vec<SourcePipeline>,
    pub input: RecordBatch,
    pub execution: Vec<PipelineExecution>,
}

pub enum ViewState {
    Module(module::ModuleView),
    InputTable(table::TableView),
    OutputTable(output::OutputView),
}

pub struct App {
    pub cache: Arc<CacheManager>,
    pub builder: Builder,
    pub pipeline: PipelineState,
    pub view: ViewState,
    pub status_msg: String,
    pub quit: bool,
}

impl App {
    pub async fn load(yaml_path: &Path, input_path: &Path) -> Result<Self> {
        let cache = Arc::new(CacheManager::from_env().await?);
        let builder = Builder::from_env(cache.clone()).await.context("Failed to initialize builder")?;

        let tui_path = yaml_path.with_extension("tui.json");
        let pipelines: Vec<SourcePipeline> = if tui_path.exists() {
            let json = std::fs::read_to_string(&tui_path)?;
            serde_json::from_str(&json).unwrap_or_default()
        } else {
            let config_str = fs::read_to_string(yaml_path)?;
            // The yaml might be a single PipelineConfig or a Map of them
            let mut configs = HashMap::new();
            if let Ok(config) =
                serde_yaml::from_str::<pyroduct::pipeline::PipelineConfig>(&config_str)
            {
                configs.insert("default".to_string(), config);
            } else if let Ok(map) = serde_yaml::from_str::<
                HashMap<String, pyroduct::pipeline::PipelineConfig>,
            >(&config_str)
            {
                configs = map;
            } else {
                anyhow::bail!("Failed to parse pipeline configuration as single config or map");
            }

            let mut pipelines = Vec::new();
            for (name, pipeline_config) in configs {
                let playbook = pipeline_config.playbook;
                if let Ok(source_module) = cache.get_source(&playbook.hash).await {
                    let mut capabilities = HashMap::new();
                    for cap in source_module.dependencies.capabilities {
                        let config = playbook.configurations.get(&cap.package).cloned().flatten();
                        capabilities.insert(cap, config);
                    }
                    pipelines.push(SourcePipeline {
                        name,
                        source: source_module.source,
                        compile_error: None,
                        capabilities,
                        wal_capacity: pipeline_config.wal_capacity,
                        success_log_retention_secs: pipeline_config.success_log_retention_secs,
                        error_log_retention_secs: pipeline_config.error_log_retention_secs,
                    });
                } else {
                    pipelines.push(SourcePipeline {
                        name,
                        source: String::new(),
                        compile_error: Some("Failed to load source".to_string()),
                        capabilities: HashMap::new(),
                        wal_capacity: pipeline_config.wal_capacity,
                        success_log_retention_secs: pipeline_config.success_log_retention_secs,
                        error_log_retention_secs: pipeline_config.error_log_retention_secs,
                    });
                }
            }
            pipelines
        };

        let input_batch = Self::load_input(input_path).await?;

        let (initial_code, initial_caps) = if let Some(pipeline) = pipelines.first() {
            let caps: Vec<(String, String)> = pipeline
                .capabilities
                .iter()
                .map(|(k, v)| {
                    (
                        k.package.clone(),
                        serde_json::to_string(v).unwrap_or_default(),
                    )
                })
                .collect();
            (pipeline.source.clone(), caps)
        } else {
            (String::new(), Vec::new())
        };

        let code_state = wasm::CodeState {
            editing: false,
            area: Rect::default(),
            editor: Editor::new("rust", &initial_code, vesper()),
            selected_pipeline: 0,
        };
        let cap_state = cap_config::CapConfigState::new(initial_caps);

        Ok(App {
            cache,
            builder,
            pipeline: PipelineState {
                yaml_path: yaml_path.to_path_buf(),
                pipelines,
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
        let batches = pyro_file::parse_data_to_batch(bytes, &filename).await?;
        if batches.is_empty() {
            Ok(RecordBatch::new_empty(Arc::new(Schema::empty())))
        } else {
            Ok(batches[0].clone().to_batch())
        }
    }

    pub async fn save(&mut self) {
        if let ViewState::Module(mv) = &mut self.view {
            let pipeline_idx = mv.selected_pipeline();
            if let Some(p) = self.pipeline.pipelines.get_mut(pipeline_idx) {
                p.source = mv.code.editor.get_content();
                self.status_msg = "Saved source code".into();
            } else {
                self.status_msg = "Error: selected pipeline not found".into();
                return;
            }

            // Persistence
            let tui_path = self.pipeline.yaml_path.with_extension("tui.json");
            let json = serde_json::to_string_pretty(&self.pipeline.pipelines).unwrap_or_default();
            if let Err(e) = std::fs::write(&tui_path, json) {
                self.status_msg = format!("Failed to save TUI state: {}", e);
            }
        } else {
            self.status_msg = "Nothing to save here.".into();
        }
    }

    async fn run_pipeline_inner(&mut self) -> Result<()> {
        let mut pipelines = Vec::new();
        let output_dir = self.pipeline.yaml_path.parent().unwrap_or(Path::new("."));

        for (i, source_pipeline) in self.pipeline.pipelines.iter().enumerate() {
            let mut configurations: HashMap<String, Option<serde_json::Value>> = HashMap::new();
            let mut capabilities = Vec::new();

            for (cap, config) in &source_pipeline.capabilities {
                capabilities.push(cap.clone());
                configurations.insert(cap.package.clone(), config.clone());
            }

            let dependencies = pyro_artifacts::artifacts::ModuleDependencies {
                dependencies: std::collections::BTreeMap::new(),
                capabilities,
            };

            let module_source = pyro_artifacts::artifacts::ModuleSource {
                dependencies,
                source: source_pipeline.source.clone(),
                ident: None,
            };

            let binary = self
                .builder
                .compile(&module_source)
                .await
                .context(format!("Compilation failed for pipeline {}", i))?;

            let playbook = pyro_artifacts::artifacts::Playbook {
                hash: binary.hash(),
                configurations,
            };

            let pipeline_config = pyroduct::pipeline::PipelineConfig {
                playbook,
                wal_capacity: source_pipeline.wal_capacity,
                success_log_retention_secs: source_pipeline.success_log_retention_secs,
                error_log_retention_secs: source_pipeline.error_log_retention_secs,
                output_dir: output_dir.to_path_buf(),
            };

            let loaded = pipeline_config
                .load(self.cache.as_ref())
                .await
                .map_err(|e| anyhow::anyhow!("Load failed: {:?}", e))?;
            let factory = loaded.factory()?;
            pipelines.push(factory.build().await?);
        }

        let pool = PipelinePool::new(pipelines);

        let (successes, failures) = pool.process_batch(&self.pipeline.input).await?;

        let mut all_executions = successes;
        all_executions.extend(failures);
        all_executions.sort_by_key(|e| e.row_index);
        self.pipeline.execution = all_executions;

        // If currently viewing the output table, refresh it seamlessly
        if let ViewState::OutputTable(ov) = &mut self.view {
            let pipeline_idx = ov.pipeline_idx;
            if let Ok(new_ov) =
                crate::tui::output::OutputView::new(self.pipeline.execution.clone(), pipeline_idx)
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
            ViewState::Module(mv) => (mv.selected_pipeline() * 2) + 1,
            ViewState::OutputTable(_) => (self.pipeline.pipelines.len() * 2) + 1,
        }
    }

    pub fn nav_to(&mut self, new_idx: usize) {
        let max_idx = (self.pipeline.pipelines.len() * 2) + 1;
        if new_idx > max_idx {
            return;
        }

        // Sync old code text to domain before navigating away
        if let ViewState::Module(mv) = &self.view {
            let pipeline_idx = mv.selected_pipeline();
            if let Some(p) = self.pipeline.pipelines.get_mut(pipeline_idx) {
                p.source = mv.code.editor.get_content();
            }
        }

        // Hydrate the new view from the domain state
        if new_idx == 0 {
            self.view = ViewState::InputTable(table::TableView::new(self.pipeline.input.clone()));
        } else {
            let pipeline_idx = (new_idx - 1) / 2;
            let is_code = new_idx % 2 == 1;
            if is_code {
                let Some(pipeline) = self.pipeline.pipelines.get(pipeline_idx) else {
                    return;
                };
                let source = pipeline.source.clone();
                let code_state = wasm::CodeState {
                    editing: false,
                    area: Rect::default(),
                    editor: Editor::new("rust", &source, vesper()),
                    selected_pipeline: pipeline_idx,
                };

                let initial_caps: Vec<(String, String)> = pipeline
                    .capabilities
                    .iter()
                    .map(|(k, v)| {
                        (
                            k.package.clone(),
                            serde_json::to_string(v).unwrap_or_default(),
                        )
                    })
                    .collect();

                let cap_state = cap_config::CapConfigState::new(initial_caps);
                self.view = ViewState::Module(module::ModuleView::new(code_state, cap_state));
            } else {
                self.view = ViewState::OutputTable(
                    output::OutputView::new(self.pipeline.execution.clone(), pipeline_idx + 1)
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
            mv.render(f, main_area);
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
                            mv.cap_config.refresh_available_caps(app.cache.as_ref()).await;
                            if mv.cap_config.selected_tab < mv.cap_config.editors.len() {
                                let name =
                                    mv.cap_config.editors[mv.cap_config.selected_tab].0.clone();
                                mv.cap_config.load_interface(app.cache.as_ref(), &name).await;
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
                mv.handle_event(key, app.cache.as_ref()).await?;
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
