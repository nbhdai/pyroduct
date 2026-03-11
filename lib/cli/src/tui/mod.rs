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

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use pyroduct::pipeline::wasm_execute::PipelineExecution;
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
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

pub struct ModuleStep {
    pub name: String,
    pub path: PathBuf,
    pub source_code: String,
    pub cap_configs: Vec<(String, String)>,
}

pub struct PipelineState {
    pub yaml_path: PathBuf,
    pub steps: Vec<ModuleStep>,
    pub input: RecordBatch,
    pub execution: Vec<PipelineExecution>,
}

pub enum ViewState {
    Module(module::ModuleView),
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
    pub async fn load(yaml_path: &Path, input_path: &Path) -> Result<Self> {
        let config = crate::run::load_config(yaml_path)?;

        let mut steps = Vec::new();
        for mod_conf in &config.pipeline {
            let path = mod_conf.path.clone();
            let name = path
                .components()
                .last()
                .expect("Non empty path")
                .as_os_str()
                .display()
                .to_string();
            let src_path = path.join("src/lib.rs");

            let source_code = fs::read_to_string(&src_path)
                .unwrap_or_else(|_| "// No source found\n".to_string());

            // Extract capability configs as (name, yaml_string) pairs
            let cap_configs: Vec<(String, String)> = mod_conf
                .configurations
                .iter()
                .map(|(name, value)| {
                    let yaml = serde_yaml::to_string(value).unwrap_or_default();
                    (name.clone(), yaml)
                })
                .collect();

            steps.push(ModuleStep {
                name,
                path,
                source_code,
                cap_configs,
            });
        }

        let initial_code = steps
            .first()
            .map(|s| s.source_code.clone())
            .unwrap_or_default();
        let initial_caps = steps
            .first()
            .map(|s| s.cap_configs.clone())
            .unwrap_or_default();

        let mut input_batch = RecordBatch::new_empty(Arc::new(Schema::empty()));
        if input_path.exists() {
            let bytes = fs::read(input_path)?;
            let filename = input_path.file_name().unwrap_or_default().to_string_lossy();
            let batches = arrow_file::parse_data_to_batch(bytes, &filename).await?;
            if !batches.is_empty() {
                input_batch = batches[0].clone().to_batch();
            }
        } else {
            anyhow::bail!("Input file does not exist: {}", input_path.display());
        }

        let code_state = wasm::CodeState {
            editing: false,
            area: Rect::default(),
            editor: Editor::new("rust", &initial_code, vesper()),
            selected_step: 0,
        };
        let cap_state = cap_config::CapConfigState::new(initial_caps);

        Ok(App {
            pipeline: PipelineState {
                yaml_path: yaml_path.to_path_buf(),
                steps,
                input: input_batch,
                execution: Vec::new(),
            },
            view: ViewState::Module(module::ModuleView::new(code_state, cap_state)),
            status_msg: String::new(),
            quit: false,
        })
    }

    pub fn save(&mut self) {
        if let ViewState::Module(mv) = &mut self.view {
            let step_idx = mv.selected_step();
            self.pipeline.steps[step_idx].source_code = mv.code.editor.get_content();

            let step = &self.pipeline.steps[step_idx];
            let path = step.path.join("src/lib.rs");

            match fs::write(&path, &step.source_code) {
                Ok(_) => {
                    // Package/compile the module (capturing the output to keep TUI clean)
                    match crate::artifacts::package::package(&step.path, None, &[], true) {
                        Ok(_) => self.status_msg = format!("Saved & compiled {}", step.name),
                        Err(e) => self.status_msg = format!("Saved, but compilation failed: {}", e),
                    }
                }
                Err(e) => {
                    self.status_msg = format!("Save failed: {}", e);
                }
            }
        } else {
            self.status_msg = "Nothing to save here.".into();
        }
    }

    async fn run_pipeline_inner(&mut self) -> Result<()> {
        let config = crate::run::load_config(&self.pipeline.yaml_path)?;
        let mut factory = pyroduct::pipeline::PipelineFactory::load(&config).await?;
        let pipeline = factory.build().await?;
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
        self.save();
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
            ViewState::OutputTable(_) => self.pipeline.steps.len() + 1,
        }
    }

    pub fn nav_to(&mut self, new_idx: usize) {
        let max_idx = self.pipeline.steps.len() + 1;
        if new_idx > max_idx {
            return;
        }

        // Sync old code text to domain before navigating away
        if let ViewState::Module(mv) = &self.view {
            self.pipeline.steps[mv.selected_step()].source_code = mv.code.editor.get_content();
        }

        // Hydrate the new view from the domain state
        if new_idx == 0 {
            self.view = ViewState::InputTable(table::TableView::new(self.pipeline.input.clone()));
        } else {
            let stage_idx = (new_idx - 1) / 2;
            let is_code = new_idx % 2 == 1;
            if is_code {
                let step = &self.pipeline.steps[stage_idx];
                let code_state = wasm::CodeState {
                    editing: false,
                    area: Rect::default(),
                    editor: Editor::new("rust", &step.source_code, vesper()),
                    selected_step: stage_idx,
                };
                let cap_state = cap_config::CapConfigState::new(step.cap_configs.clone());
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
    let hk = app.hotkeys();
    keys::render(f, &hk, area);
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
            app.save();
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
