use std::{
    collections::HashMap,
    fs,
    io::stdout,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, Tabs, Wrap},
};
use ratatui_code_editor::editor::Editor;
use ratatui_code_editor::theme::vesper;
use serde::Deserialize;

// =============================================================================
// Pipeline YAML (load once to bootstrap)
// =============================================================================

#[derive(Deserialize, Debug, Clone)]
struct PipelineYaml {
    #[serde(default)]
    capabilities: HashMap<String, CapabilityYaml>,
    modules: HashMap<String, ModuleYaml>,
    pipeline: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
struct CapabilityYaml {
    path: String,
    #[serde(default)]
    classes: HashMap<String, serde_yaml::Value>,
}

#[derive(Deserialize, Debug, Clone)]
struct ModuleYaml {
    path: String,
    #[serde(default)]
    capabilities: Vec<String>,
}

// =============================================================================
// Domain types
// =============================================================================

struct CapConfig {
    display_name: String,
    editor: Editor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StepStatus { Idle, Building, Success, Failed }

struct LogLine { text: String, level: LogLevel }

#[derive(Clone, Copy)]
enum LogLevel { Info, Warn, Error }

struct DataRow { columns: Vec<(String, String)> }

struct ModuleStep {
    name: String,
    source_path: Option<PathBuf>,
    wasm_path: PathBuf,
    code: Editor,
    capabilities: Vec<CapConfig>,
    logs: Vec<LogLine>,
    status: StepStatus,
}

// =============================================================================
// Pane focus
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pane { Pipeline, Capabilities, Code, Logs, Data }

impl Pane {
    fn next(self) -> Self {
        match self {
            Pane::Pipeline => Pane::Capabilities,
            Pane::Capabilities => Pane::Code,
            Pane::Code => Pane::Logs,
            Pane::Logs => Pane::Data,
            Pane::Data => Pane::Pipeline,
        }
    }
    fn prev(self) -> Self {
        match self {
            Pane::Pipeline => Pane::Data,
            Pane::Capabilities => Pane::Pipeline,
            Pane::Code => Pane::Capabilities,
            Pane::Logs => Pane::Code,
            Pane::Data => Pane::Logs,
        }
    }
    fn label(&self) -> &'static str {
        match self {
            Pane::Pipeline => "pipeline",
            Pane::Capabilities => "capabilities",
            Pane::Code => "code",
            Pane::Logs => "logs",
            Pane::Data => "data",
        }
    }
}

// =============================================================================
// App
// =============================================================================

struct App {
    modules_base_dir: PathBuf,
    steps: Vec<ModuleStep>,
    selected_step: usize,
    selected_cap: usize,
    focus: Pane,
    editing: bool,
    log_scroll: u16,
    data_scroll: usize,
    data_columns: Vec<String>,
    data_rows: Vec<DataRow>,
    status_msg: String,
    quit: bool,
    // Cached rects so editor.input() can use them
    code_area: Rect,
    cap_area: Rect,
}

impl App {
    fn load(yaml_path: &Path) -> Result<Self> {
        let content = fs::read_to_string(yaml_path)?;
        let config: PipelineYaml = serde_yaml::from_str(&content)?;
        let yaml_dir = yaml_path.parent().unwrap_or(Path::new(".")).to_path_buf();

        let mut steps = Vec::new();
        for module_name in &config.pipeline {
            let mod_conf = config.modules.get(module_name)
                .ok_or_else(|| anyhow::anyhow!("Module '{}' not in [modules]", module_name))?;

            let wasm_path = yaml_dir.join(&mod_conf.path);
            let (source_code, source_path) = find_source(&wasm_path);

            let code = Editor::new("rust", &source_code, vesper());

            let mut caps = Vec::new();
            for cap_name in &mod_conf.capabilities {
                if let Some(cap_yaml) = config.capabilities.get(cap_name) {
                    for (class_name, class_val) in &cap_yaml.classes {
                        let yaml_str = serde_yaml::to_string(class_val).unwrap_or_default();
                        caps.push(CapConfig {
                            display_name: format!("{}/{}", cap_name, class_name),
                            editor: Editor::new("yaml", &yaml_str, vesper()),
                        });
                    }
                }
            }

            steps.push(ModuleStep {
                name: module_name.clone(),
                source_path,
                wasm_path,
                code,
                capabilities: caps,
                logs: vec![LogLine {
                    text: format!("── Module '{}' loaded ──", module_name),
                    level: LogLevel::Info,
                }],
                status: StepStatus::Idle,
            });
        }

        let modules_base_dir = yaml_dir.join("modules");
        let modules_base_dir = if modules_base_dir.exists() {
            modules_base_dir
        } else {
            yaml_dir.join("..").join("modules")
        };

        Ok(App {
            modules_base_dir: fs::canonicalize(&modules_base_dir).unwrap_or(modules_base_dir),
            steps,
            selected_step: 0,
            selected_cap: 0,
            focus: Pane::Code,
            editing: false,
            log_scroll: 0,
            data_scroll: 0,
            data_columns: vec![],
            data_rows: vec![],
            status_msg: "Tab: pane │ i/Enter: edit │ Esc: nav │ ^S: save │ ^R: run │ ^N: new module │ ^Q: quit".into(),
            quit: false,
            code_area: Rect::default(),
            cap_area: Rect::default(),
        })
    }

    fn current_step(&self) -> &ModuleStep { &self.steps[self.selected_step] }
    fn current_step_mut(&mut self) -> &mut ModuleStep { &mut self.steps[self.selected_step] }

    fn save(&mut self) {
        let step = &self.steps[self.selected_step];
        if let Some(path) = &step.source_path {
            let content = step.code.get_content();
            match fs::write(path, &content) {
                Ok(_) => {
                    self.status_msg = format!("Saved {}", path.display());
                    self.steps[self.selected_step].logs.push(LogLine {
                        text: format!("[save] {} bytes → {}", content.len(), path.display()),
                        level: LogLevel::Info,
                    });
                }
                Err(e) => {
                    self.status_msg = format!("Save failed: {}", e);
                    self.steps[self.selected_step].logs.push(LogLine {
                        text: format!("[save] ERROR: {}", e),
                        level: LogLevel::Error,
                    });
                }
            }
        } else {
            self.status_msg = "No source file path for this module".into();
        }
    }

    fn new_module(&mut self) {
        let n = self.steps.len() + 1;
        let name = format!("module_{}", n);
        let mod_dir = self.modules_base_dir.join(&name);
        let src_dir = mod_dir.join("src");

        if let Err(e) = fs::create_dir_all(&src_dir) {
            self.status_msg = format!("mkdir failed: {}", e);
            return;
        }

        let cargo_toml = format!(
r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
pyroduct = {{ path = "../../lib/pyroduct" }}
anyhow = "1"
"#);
        if let Err(e) = fs::write(mod_dir.join("Cargo.toml"), &cargo_toml) {
            self.status_msg = format!("Write Cargo.toml failed: {}", e);
            return;
        }

        let lib_rs = format!(
r#"use pyroduct::prelude::*;

#[module(output = result)]
fn {name}(input: &str) -> anyhow::Result<String> {{
    Ok(format!("[{name}] {{}}", input))
}}
"#);
        let lib_path = src_dir.join("lib.rs");
        if let Err(e) = fs::write(&lib_path, &lib_rs) {
            self.status_msg = format!("Write lib.rs failed: {}", e);
            return;
        }

        let code = Editor::new("rust", &lib_rs, vesper());

        self.steps.push(ModuleStep {
            name: name.clone(),
            source_path: Some(lib_path),
            wasm_path: mod_dir.join("artifacts").join("mod.wasm"),
            code,
            capabilities: vec![],
            logs: vec![LogLine {
                text: format!("── Scaffolded '{}' at {} ──", name, mod_dir.display()),
                level: LogLevel::Info,
            }],
            status: StepStatus::Idle,
        });

        self.selected_step = self.steps.len() - 1;
        self.selected_cap = 0;
        self.status_msg = format!("Created module '{}'", name);
    }

    fn run(&mut self) {
        // Save all
        for step in &self.steps {
            if let Some(path) = &step.source_path {
                let _ = fs::write(path, step.code.get_content());
            }
        }

        for step in &mut self.steps {
            step.status = StepStatus::Building;
            step.logs.push(LogLine {
                text: format!("[build] cargo build --target wasm32-unknown-unknown -p {} ...", step.name),
                level: LogLevel::Info,
            });
        }

        // TODO: real impl would invoke:
        //   cargo build --target wasm32-unknown-unknown for each module
        //   PipelineDef::load(&config).await
        //   Pipeline::new(def).await
        //   for row in input_data { pipeline.process(row).await }
        //
        // Simulated results:
        for step in &mut self.steps {
            step.status = StepStatus::Success;
            step.logs.push(LogLine {
                text: format!("[run]   '{}' OK", step.name),
                level: LogLevel::Info,
            });
        }

        self.data_columns = vec!["row".into(), "input".into(), "output".into(), "status".into()];
        self.data_rows = vec![
            DataRow { columns: vec![
                ("row".into(), "0".into()), ("input".into(), "hello".into()),
                ("output".into(), "[TEST] HELLO!!!".into()), ("status".into(), "ok".into()),
            ]},
            DataRow { columns: vec![
                ("row".into(), "1".into()), ("input".into(), "world".into()),
                ("output".into(), "[TEST] WORLD!!!".into()), ("status".into(), "ok".into()),
            ]},
        ];
        self.data_scroll = 0;
        self.status_msg = format!("Pipeline complete. {} rows.", self.data_rows.len());
    }

    fn remove_step(&mut self) {
        if self.steps.len() > 1 {
            let name = self.steps[self.selected_step].name.clone();
            self.steps.remove(self.selected_step);
            if self.selected_step >= self.steps.len() {
                self.selected_step = self.steps.len() - 1;
            }
            self.status_msg = format!("Removed '{}'", name);
        }
    }

    fn add_capability(&mut self) {
        let n = self.current_step().capabilities.len() + 1;
        let editor = Editor::new("yaml", "library: <name>\nkey: value\n", vesper());
        self.current_step_mut().capabilities.push(CapConfig {
            display_name: format!("cap_{}/class", n),
            editor,
        });
        self.selected_cap = self.current_step().capabilities.len() - 1;
    }
}

// =============================================================================
// Source file discovery
// =============================================================================

fn find_source(wasm_path: &Path) -> (String, Option<PathBuf>) {
    let mut search = wasm_path.parent().map(|p| p.to_path_buf());
    for _ in 0..5 {
        if let Some(dir) = &search {
            for candidate in &["src/lib.rs", "src/main.rs"] {
                let src = dir.join(candidate);
                if src.exists() {
                    if let Ok(code) = fs::read_to_string(&src) {
                        let abs = fs::canonicalize(&src).unwrap_or(src);
                        return (code, Some(abs));
                    }
                }
            }
            search = dir.parent().map(|p| p.to_path_buf());
        }
    }
    (format!("// No source found for {}\n", wasm_path.display()), None)
}

// =============================================================================
// Rendering
// =============================================================================

fn ui(f: &mut Frame, app: &mut App) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(1)])
        .split(f.area());

    let h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(22), Constraint::Min(40)])
        .split(root[0]);

    render_pipeline_list(f, app, h[0]);

    let cap_h: u16 = if app.current_step().capabilities.is_empty() { 4 } else { 10 };

    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(cap_h),
            Constraint::Min(8),
            Constraint::Length(12),
        ])
        .split(h[1]);

    render_capabilities(f, app, v[0]);
    render_code(f, app, v[1]);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(v[2]);

    render_logs(f, app, bottom[0]);
    render_data_table(f, app, bottom[1]);
    render_status(f, app, root[1]);

    // Set cursor position if editing
    if app.editing {
        match app.focus {
            Pane::Code => {
                if let Some((x, y)) = app.steps[app.selected_step].code.get_visible_cursor(&app.code_area) {
                    f.set_cursor_position(Position::new(x, y));
                }
            }
            Pane::Capabilities => {
                if let Some(cap) = app.steps[app.selected_step].capabilities.get(app.selected_cap) {
                    if let Some((x, y)) = cap.editor.get_visible_cursor(&app.cap_area) {
                        f.set_cursor_position(Position::new(x, y));
                    }
                }
            }
            _ => {}
        }
    }
}

fn bstyle(focused: bool) -> Style {
    if focused { Style::default().fg(Color::Cyan) } else { Style::default().fg(Color::DarkGray) }
}

fn render_pipeline_list(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Pane::Pipeline;
    let items: Vec<ListItem> = app.steps.iter().enumerate().map(|(i, s)| {
        let icon = match s.status {
            StepStatus::Idle => "○",
            StepStatus::Building => "◌",
            StepStatus::Success => "●",
            StepStatus::Failed => "✗",
        };
        let style = if i == app.selected_step {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        ListItem::new(format!(" {} {} {}", i + 1, icon, s.name)).style(style)
    }).collect();

    let list = List::new(items).block(
        Block::default().borders(Borders::ALL).title(" Pipeline ").border_style(bstyle(focused))
    );
    let mut state = ListState::default();
    state.select(Some(app.selected_step));
    f.render_stateful_widget(list, area, &mut state);
}

fn render_capabilities(f: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == Pane::Capabilities;
    let step = &app.steps[app.selected_step];

    if step.capabilities.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Capabilities (^A to add) ")
            .border_style(bstyle(focused));
        f.render_widget(
            Paragraph::new("  No capabilities.")
                .style(Style::default().fg(Color::DarkGray))
                .block(block),
            area,
        );
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(2)])
        .split(area);

    let tab_titles: Vec<Line> = step.capabilities.iter()
        .map(|c| Line::from(c.display_name.as_str())).collect();

    let tabs = Tabs::new(tab_titles)
        .select(app.selected_cap)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .divider("│")
        .block(Block::default()
            .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
            .title(" Capabilities ")
            .border_style(bstyle(focused)));
    f.render_widget(tabs, chunks[0]);

    // Render the editor widget for the selected cap
    let editor_area = chunks[1];
    app.cap_area = editor_area;
    let cap = &app.steps[app.selected_step].capabilities[app.selected_cap];
    f.render_widget(&cap.editor, editor_area);
}

fn render_code(f: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == Pane::Code;
    let step = &app.steps[app.selected_step];

    let src_label = step.source_path.as_ref()
        .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
        .unwrap_or_else(|| "no source".into());

    // Draw a border block around the area, then render editor inside the inner area
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ─ {} ", step.name, src_label))
        .border_style(bstyle(focused));
    let inner = block.inner(area);
    f.render_widget(block, area);

    app.code_area = inner;
    f.render_widget(&app.steps[app.selected_step].code, inner);
}

fn render_logs(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Pane::Logs;
    let lines: Vec<Line> = app.current_step().logs.iter().map(|l| {
        let color = match l.level {
            LogLevel::Error => Color::Red,
            LogLevel::Warn => Color::Yellow,
            LogLevel::Info => Color::Gray,
        };
        Line::from(Span::styled(l.text.as_str(), Style::default().fg(color)))
    }).collect();

    f.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(" Logs ").border_style(bstyle(focused)))
            .scroll((app.log_scroll, 0))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_data_table(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Pane::Data;

    if app.data_columns.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Data (^R to run) ")
            .border_style(bstyle(focused));
        f.render_widget(
            Paragraph::new("  No data yet.")
                .style(Style::default().fg(Color::DarkGray))
                .block(block),
            area,
        );
        return;
    }

    let header_cells = app.data_columns.iter().map(|h| {
        Cell::from(h.as_str()).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    });
    let header = Row::new(header_cells).height(1);

    let rows = app.data_rows.iter().skip(app.data_scroll).map(|dr| {
        let cells = app.data_columns.iter().map(|col| {
            let val = dr.columns.iter()
                .find(|(k, _)| k == col)
                .map(|(_, v)| v.as_str())
                .unwrap_or("");
            let style = if val == "ok" {
                Style::default().fg(Color::Green)
            } else if val.contains("error") || val.contains("FAIL") {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::White)
            };
            Cell::from(val).style(style)
        });
        Row::new(cells)
    });

    let widths: Vec<Constraint> = app.data_columns.iter().map(|_| Constraint::Min(10)).collect();
    let table = Table::new(rows, &widths)
        .header(header)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(format!(" Data ({} rows) ", app.data_rows.len()))
            .border_style(bstyle(focused)));
    f.render_widget(table, area);
}

fn render_status(f: &mut Frame, app: &App, area: Rect) {
    let mode = if app.editing {
        Span::styled(" EDIT ", Style::default().fg(Color::Black).bg(Color::Green))
    } else {
        Span::styled(" NAV ", Style::default().fg(Color::Black).bg(Color::Blue))
    };
    let line = Line::from(vec![
        mode,
        Span::raw(" "),
        Span::styled(format!(" {} ", app.focus.label()), Style::default().fg(Color::Cyan)),
        Span::raw(" │ "),
        Span::styled(&app.status_msg, Style::default().fg(Color::DarkGray)),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

// =============================================================================
// Input
// =============================================================================

fn handle_event(app: &mut App) -> Result<()> {
    if !event::poll(Duration::from_millis(50))? { return Ok(()); }
    let ev = event::read()?;
    let Event::Key(key) = ev else { return Ok(()) };

    // ── Global hotkeys ──────────────────────────────────────────────
    match (key.modifiers, key.code) {
        (KeyModifiers::CONTROL, KeyCode::Char('c'))
        | (KeyModifiers::CONTROL, KeyCode::Char('q')) => { app.quit = true; return Ok(()); }
        (KeyModifiers::CONTROL, KeyCode::Char('s')) => { app.save(); return Ok(()); }
        (KeyModifiers::CONTROL, KeyCode::Char('r')) => { app.editing = false; app.run(); return Ok(()); }
        (KeyModifiers::CONTROL, KeyCode::Char('n')) => { app.new_module(); return Ok(()); }
        (KeyModifiers::CONTROL, KeyCode::Char('d')) => { app.remove_step(); return Ok(()); }
        (KeyModifiers::CONTROL, KeyCode::Char('a')) => { app.add_capability(); return Ok(()); }
        _ => {}
    }

    // ── Edit mode: forward to editor ────────────────────────────────
    if app.editing {
        if key.code == KeyCode::Esc {
            app.editing = false;
            app.status_msg = "Navigation mode.".into();
            return Ok(());
        }
        match app.focus {
            Pane::Code => {
                let area = app.code_area;
                app.steps[app.selected_step].code.input(key, &area)?;
            }
            Pane::Capabilities => {
                if let Some(cap) = app.steps[app.selected_step].capabilities.get_mut(app.selected_cap) {
                    let area = app.cap_area;
                    cap.editor.input(key, &area)?;
                }
            }
            _ => { app.editing = false; }
        }
        return Ok(());
    }

    // ── Navigation mode ─────────────────────────────────────────────
    match key.code {
        KeyCode::Tab => app.focus = app.focus.next(),
        KeyCode::BackTab => app.focus = app.focus.prev(),
        KeyCode::Enter | KeyCode::Char('i') => {
            if app.focus == Pane::Code || app.focus == Pane::Capabilities {
                app.editing = true;
                app.status_msg = "Editing ─ Esc to return".into();
            }
        }
        _ => {}
    }

    match app.focus {
        Pane::Pipeline => match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if app.selected_step > 0 {
                    app.selected_step -= 1;
                    app.selected_cap = 0;
                    app.log_scroll = 0;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if app.selected_step + 1 < app.steps.len() {
                    app.selected_step += 1;
                    app.selected_cap = 0;
                    app.log_scroll = 0;
                }
            }
            _ => {}
        },
        Pane::Capabilities => match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
                if app.selected_cap > 0 { app.selected_cap -= 1; }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                let max = app.current_step().capabilities.len().saturating_sub(1);
                if app.selected_cap < max { app.selected_cap += 1; }
            }
            _ => {}
        },
        Pane::Logs => match key.code {
            KeyCode::Up | KeyCode::Char('k') => { app.log_scroll = app.log_scroll.saturating_sub(1); }
            KeyCode::Down | KeyCode::Char('j') => { app.log_scroll += 1; }
            _ => {}
        },
        Pane::Data => match key.code {
            KeyCode::Up | KeyCode::Char('k') => { app.data_scroll = app.data_scroll.saturating_sub(1); }
            KeyCode::Down | KeyCode::Char('j') => {
                if app.data_scroll + 1 < app.data_rows.len() { app.data_scroll += 1; }
            }
            _ => {}
        },
        Pane::Code => {}
    }

    Ok(())
}

// =============================================================================
// Main
// =============================================================================

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let yaml_path = args.get(1).map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("pipeline.yaml"));

    if !yaml_path.exists() {
        eprintln!("Usage: pyroduct-tui <pipeline.yaml>");
        eprintln!("  File not found: {}", yaml_path.display());
        std::process::exit(1);
    }

    let mut app = App::load(&yaml_path)?;

    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|f| ui(f, &mut app))?;
        handle_event(&mut app)?;
        if app.quit { break; }
    }

    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}