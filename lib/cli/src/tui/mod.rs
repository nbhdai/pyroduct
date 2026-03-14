// lib/cli/src/tui/mod.rs
use std::{
    collections::{BTreeMap, HashMap},
    fs,
    io::stdout,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use arrow::array::RecordBatch;
use arrow::datatypes::Schema;

use anyhow::{Context, Result};
use cargo_toml::Dependency;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers}, execute, terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode}
};
use pyroduct::pipeline::wasm_execute::{Pipeline, PipelineExecution};
use ratatui::{
    prelude::*,
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect}, style::Style,
};
use ratatui_code_editor::{editor::Editor, theme::vesper};
use serde::{Deserialize, Serialize};

use artifacts::{
    artifacts::Module,
    cache::CacheManager,
    cargo::{CapabilityManifest, ModuleManifest, ResolvedCapability},
    environment::dylib_extension,
    build::CommandError,
};
use pyroduct::module::{PyroFactory, PyroModule, capability::CapabilityLibrary};

pub mod cap_config;
pub mod keys;
pub mod logs;
pub mod module;
pub mod nav;
pub mod output;
pub mod table;
pub mod wasm;

use keys::{Hotkey, HotkeyProvider};

#[derive(Serialize, Deserialize, Clone)]
pub struct CompleteModule {
    pub name: String,
    pub path: PathBuf,
    pub source_code: String,
    pub dependencies: BTreeMap<String, Dependency>,
    pub capabilities: Vec<ResolvedCapability>,

    pub configurations: HashMap<String, Option<serde_json::Value>>,
    pub artifact: Option<Module>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TuiPipeline {
    pub modules: Vec<CompleteModule>,
}

impl TuiPipeline {
    pub async fn from_config(yaml_path: &Path) -> Result<Self> {
        let config = crate::run::load_config(yaml_path)?;
        let mut modules = Vec::new();

        for (name, mod_conf) in config.pipeline {
            let src_path = mod_conf.path.join("src/lib.rs");
            let source_code = fs::read_to_string(&src_path)
                .unwrap_or_else(|_| "// No source found\n".to_string());

            let mut dependencies = BTreeMap::new();
            let mod_toml_path = mod_conf.path.join("Module.toml");
            if mod_toml_path.exists() {
                let toml_content = fs::read_to_string(&mod_toml_path)?;
                let manifest: ModuleManifest = toml::from_str(&toml_content)?;
                dependencies = manifest.dependencies;
            }

            let mut capabilities = Vec::new();
            for lib_path in &mod_conf.libraries {
                let cap_toml_path = lib_path.join("Capability.toml");
                if cap_toml_path.exists() {
                    let toml_content = fs::read_to_string(&cap_toml_path)?;
                    let manifest: CapabilityManifest = toml::from_str(&toml_content)?;
                    let author = manifest.capability.author.clone();
                    let package = manifest.capability.name.clone();
                    let version = manifest.capability.version.clone();

                    capabilities.push(ResolvedCapability {
                        author,
                        package,
                        version,
                    });
                }
            }

            modules.push(CompleteModule {
                name,
                path: mod_conf.path,
                source_code,
                dependencies,
                capabilities,
                configurations: mod_conf.configurations,
                artifact: None,
            });
        }

        Ok(TuiPipeline { modules })
    }

    pub fn from_json(json_path: &Path) -> Result<Self> {
        let content = fs::read_to_string(json_path)?;
        let pipeline = serde_json::from_str::<TuiPipeline>(&content)?;
        Ok(pipeline)
    }

    pub fn save(&self, json_path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(json_path, json)?;
        Ok(())
    }
}

pub struct PipelineState {
    pub yaml_path: PathBuf,
    pub tui: TuiPipeline,
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
        let cache = CacheManager::new().await?;
        let tui_path = yaml_path.with_extension("tui.json");
        let tui_pipeline = if tui_path.exists() {
            TuiPipeline::from_json(&tui_path)?
        } else {
            TuiPipeline::from_config(yaml_path).await?
        };

        let initial_code = tui_pipeline
            .modules
            .first()
            .map(|m| m.source_code.clone())
            .unwrap_or_default();
        let initial_caps: Vec<(String, String)> = tui_pipeline
            .modules
            .first()
            .map(|m| {
                m.configurations
                    .iter()
                    .map(|(k, v)| (k.clone(), serde_yaml::to_string(v).unwrap_or_default()))
                    .collect()
            })
            .unwrap_or_default();

        let input_batch = Self::load_input(input_path).await?;

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
                tui: tui_pipeline,
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
        let batches = arrow_file::parse_data_to_batch(bytes, &filename).await?;
        if batches.is_empty() {
            Ok(RecordBatch::new_empty(Arc::new(Schema::empty())))
        } else {
            Ok(batches[0].clone().to_batch())
        }
    }

    pub async fn save(&mut self) {
        if let ViewState::Module(mv) = &mut self.view {
            let step_idx = mv.selected_step();
            let mut module = self.pipeline.tui.modules[step_idx].clone();
            module.source_code = mv.code.editor.get_content();

            // Compile the module ephemerally
            let dependencies: BTreeMap<String, Dependency> = module
                .dependencies
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

            match self
                .cache
                .compile_anon(
                    &dependencies,
                    &module.capabilities,
                    &module.source_code,
                )
                .await
            {
                Ok(artifact) => {
                    module.artifact = Some(artifact);
                    self.status_msg = format!("Compiled {}", module.name);
                }
                Err(e) => {
                    self.status_msg = format!("Compilation failed: {}", e);
                }
            }

            self.pipeline.tui.modules[step_idx] = module;

            // Persist the entire TUI pipeline to JSON
            let tui_path = self.pipeline.yaml_path.with_extension("tui.json");
            if let Err(e) = self.pipeline.tui.save(&tui_path) {
                self.status_msg = format!("Failed to save TUI state: {}", e);
            }
        } else {
            self.status_msg = "Nothing to save here.".into();
        }
    }

    async fn run_pipeline_inner(&mut self) -> Result<()> {
        let mut steps = Vec::new();

        let mut config = wasmtime::Config::new();
        config.async_support(true);
        let engine = wasmtime::Engine::new(&config)
            .map_err(|e| anyhow::anyhow!("Failed to create wasmtime engine: {}", e))?;

        for (i, module) in self.pipeline.tui.modules.iter_mut().enumerate() {
            // 1. Ensure module is compiled
            if module.artifact.is_none() {
                let dependencies: BTreeMap<String, Dependency> = module
                    .dependencies
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                match self
                    .cache
                    .compile_anon(
                        &dependencies,
                        &module.capabilities,
                        &module.source_code,
                    )
                    .await
                {
                    Ok(artifact) => {
                        module.artifact = Some(artifact);
                    }
                    Err(e) => {
                        match &e {
                            artifacts::cache::BuildError::Command(CommandError::Cargo { stderr, .. }) => {
                                if let ViewState::Module(mv) = &mut self.view {
                                    mv.logs = logs::LogsView::from_stderr(&stderr);
                                    mv.bottom_tab = module::BottomTab::Logs;
                                }
                            },
                            _ => {},
                        };
                        anyhow::bail!("Module {} is not compiled: {:?}", i, e)
                    }
                }
            }

            let artifact = module.artifact.as_ref().unwrap();

            // 2. Load capabilities from cache
            let mut libs = Vec::new();
            let lib_file = format!("lib.{}", dylib_extension());

            for cap in &module.capabilities {
                let cap_dir = self
                    .cache
                    .capabilities_dir(&cap.author, &cap.package, &cap.version);
                let artifact_path = cap_dir.join(&lib_file);

                let library = CapabilityLibrary::load(cap.package.clone(), &artifact_path)
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to load capability library from cache: {}",
                            artifact_path.display()
                        )
                    })?;
                libs.push(library);
            }

            // 3. Create PyroFactory and instantiate
            let wasmtime_module = wasmtime::Module::from_binary(&engine, &artifact.wasm)
                .map_err(|e| anyhow::anyhow!("Failed to compile WASM: {}", e))?;
            let pyro_module = PyroModule::new(wasmtime_module)?;

            let mut factory = PyroFactory::new(libs, module.configurations.clone(), pyro_module)
                .map_err(|e| anyhow::anyhow!("Failed to create PyroFactory: {}", e))?;

            let instance = factory
                .instantiate()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to instantiate module: {}", e))?;
            steps.push(instance);
        }

        let pipeline = Pipeline { steps };
        let pool = pyroduct::pipeline::PipelinePool::new(vec![pipeline]);

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
            ViewState::OutputTable(_) => self.pipeline.tui.modules.len() + 1,
        }
    }

    pub fn nav_to(&mut self, new_idx: usize) {
        let max_idx = self.pipeline.tui.modules.len() + 1;
        if new_idx > max_idx {
            return;
        }

        // Sync old code text to domain before navigating away
        if let ViewState::Module(mv) = &self.view {
            self.pipeline.tui.modules[mv.selected_step()].source_code =
                mv.code.editor.get_content();
        }

        // Hydrate the new view from the domain state
        if new_idx == 0 {
            self.view = ViewState::InputTable(table::TableView::new(self.pipeline.input.clone()));
        } else {
            let stage_idx = (new_idx - 1) / 2;
            let is_code = new_idx % 2 == 1;
            if is_code {
                let module = &self.pipeline.tui.modules[stage_idx];
                let code_state = wasm::CodeState {
                    editing: false,
                    area: Rect::default(),
                    editor: Editor::new("rust", &module.source_code, vesper()),
                    selected_step: stage_idx,
                };

                let initial_caps: Vec<(String, String)> = module
                    .configurations
                    .iter()
                    .map(|(k, v)| (k.clone(), serde_yaml::to_string(v).unwrap_or_default()))
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

fn ui(f: &mut Frame, app: &mut App) {
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
        let status = ratatui::widgets::Paragraph::new(
            ratatui::text::Span::styled(&app.status_msg, style),
        );
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
                mv.handle_event(key)?;
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
