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

pub mod wasm;

pub struct ModuleStep {
    pub name: String,
    pub path: PathBuf,
    pub code: Editor,
}

pub struct App {
    pub yaml_path: PathBuf,
    pub steps: Vec<ModuleStep>,
    pub selected_step: usize,
    pub editing: bool,
    pub status_msg: String,
    pub quit: bool,
    pub code_area: Rect,
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
            
            // Read the actual file contents to populate the editor
            let source_code =
                fs::read_to_string(&src_path).unwrap_or_else(|_| "// No source found\n".to_string());

            let code = Editor::new("rust", &source_code, vesper());

            steps.push(ModuleStep {
                name: module_name.clone(),
                path,
                code,
            });
        }

        Ok(App {
            yaml_path: yaml_path.to_path_buf(),
            steps,
            selected_step: 0,
            editing: false,
            status_msg: "i/Enter: edit │ ↑/↓: switch module │ ^S: save │ ^Q: quit".into(),
            quit: false,
            code_area: Rect::default(),
        })
    }

    pub fn save(&mut self) {
        let step = &mut self.steps[self.selected_step];
        let path = step.path.join("src/lib.rs");
        let content = step.code.get_content();
        match fs::write(&path, &content) {
            Ok(_) => {
                self.status_msg = format!("Saved {}", path.display());
            }
            Err(e) => {
                self.status_msg = format!("Save failed: {}", e);
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(1)])
        .split(f.area());

    // Delegate rendering entirely to our new code view
    wasm::render(f, app, chunks[0]);

    render_status(f, app, chunks[1]);
}

fn render_status(f: &mut Frame, app: &App, area: Rect) {
    let mode = if app.editing {
        Span::styled(" EDIT ", Style::default().fg(Color::Black).bg(Color::Green))
    } else {
        Span::styled(" NAV ", Style::default().fg(Color::Black).bg(Color::Blue))
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
        _ => {}
    }

    // Pass control to the View if we're in edit mode
    if app.editing {
        wasm::handle_event(app, key)?;
        return Ok(());
    }

    // Navigation mode
    match key.code {
        KeyCode::Enter | KeyCode::Char('i') => {
            app.editing = true;
            app.status_msg = "Editing ─ Esc to return".into();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.selected_step > 0 {
                app.selected_step -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.selected_step + 1 < app.steps.len() {
                app.selected_step += 1;
            }
        }
        _ => {}
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