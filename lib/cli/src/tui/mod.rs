use std::{
    fs,
    io::stdout,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
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
pub mod wasm;

pub struct ModuleStep {
    pub name: String,
    pub path: PathBuf,
    pub source_code: String,
}

/// Global data that all views might need to access or modify
pub struct PipelineState {
    pub yaml_path: PathBuf,
    pub steps: Vec<ModuleStep>,
}

/// The state machine representing the active view in the TUI.
pub enum ViewState {
    Code(wasm::CodeState),
    // We will add more states here like `Logs(logs::LogsState)`, etc.
}

pub struct App {
    pub pipeline: PipelineState,
    pub view: ViewState,
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

        Ok(App {
            pipeline: PipelineState {
                yaml_path: yaml_path.to_path_buf(),
                steps,
            },
            view: ViewState::Code(wasm::CodeState {
                editing: false,
                area: Rect::default(),
                editor: Editor::new("rust", &initial_code, vesper()),
                selected_step: 0,
                status_msg: "i/Enter: edit │ ↑/↓: switch module │ ^S: save │ ^Q: quit".into(),
                quit: false,
            }),
        })
    }

    pub fn save(&mut self) {
        // Sync the current editor content to the domain model before saving
        // and grab the path and content to be saved.
        let (path, content) = match &mut self.view {
            ViewState::Code(code_state) => {
                let step_idx = code_state.selected_step;
                self.pipeline.steps[step_idx].source_code = code_state.editor.get_content();
                
                let step = &self.pipeline.steps[step_idx];
                (step.path.join("src/lib.rs"), step.source_code.clone())
            }
        };

        // Do the IO operation
        let save_result = fs::write(&path, &content);

        // Update the view state with the result
        match &mut self.view {
            ViewState::Code(code_state) => {
                match save_result {
                    Ok(_) => {
                        code_state.status_msg = format!("Saved {}", path.display());
                    }
                    Err(e) => {
                        code_state.status_msg = format!("Save failed: {}", e);
                    }
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    // Split vertical chunks: Top for Main App, Bottom for Status Bar
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(1)])
        .split(f.area());

    // Split horizontal chunks: Left for Code/Main View, Right for Nav
    let horizontal_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(25)]) // Nav bar is fixed at 25 chars wide
        .split(vertical_chunks[0]);

    let main_area = horizontal_chunks[0];
    let nav_area = horizontal_chunks[1];

    // Dispatch render to the active state. 
    match &mut app.view {
        ViewState::Code(code_state) => {
            wasm::render(f, &app.pipeline, code_state, main_area);
        }
    }

    // Render the nav bar directly via the app
    nav::render(f, app, nav_area);

    render_status(f, app, vertical_chunks[1]);
}

fn render_status(f: &mut Frame, app: &App, area: Rect) {
    let (is_editing, status_msg) = match &app.view {
        ViewState::Code(state) => (state.editing, &state.status_msg),
    };

    let mode = if is_editing {
        Span::styled(" EDIT ", Style::default().fg(Color::Black).bg(Color::Green))
    } else {
        Span::styled(" NAV ", Style::default().fg(Color::Black).bg(Color::Blue))
    };

    let line = Line::from(vec![
        mode,
        Span::raw(" │ "),
        Span::styled(status_msg, Style::default().fg(Color::DarkGray)),
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
            match &mut app.view {
                ViewState::Code(state) => state.quit = true,
            }
            return Ok(());
        }
        (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
            app.save();
            return Ok(());
        }
        _ => {}
    }

    // Dispatch event to the active state
    match &mut app.view {
        ViewState::Code(code_state) => {
            wasm::handle_event(&mut app.pipeline, code_state, key)?;
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
        
        let should_quit = match &app.view {
            ViewState::Code(state) => state.quit,
        };
        
        if should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}