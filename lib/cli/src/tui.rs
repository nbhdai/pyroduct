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

// =============================================================================
// Domain types & Events
// =============================================================================

pub enum TuiEvent {
    Log(usize, LogLine),
    Status(usize, StepStatus),
    RunComplete(Result<Vec<DataRow>>),
}

struct CapConfig {
    display_name: String,
    editor: Editor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStatus { Idle, Building, Success, Failed }

pub struct LogLine { text: String, level: LogLevel }

#[derive(Clone, Copy)]
pub enum LogLevel { Info, Warn, Error }

pub struct DataRow { columns: Vec<(String, String)> }

pub struct ModuleStep {
    name: String,
    path: PathBuf,
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
    yaml_path: PathBuf,
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
    
    tx: std::sync::mpsc::Sender<TuiEvent>,
    rx: std::sync::mpsc::Receiver<TuiEvent>,
}

impl App {
    fn load(yaml_path: &Path) -> Result<Self> {
        // Use the centralized config loader from the CLI
        let config = crate::run::load_config(yaml_path)?;
        let yaml_dir = yaml_path.parent().unwrap_or(Path::new(".")).to_path_buf();

        let mut steps = Vec::new();
        for module_name in &config.pipeline {
            let mod_conf = config.modules.get(module_name)
                .ok_or_else(|| anyhow::anyhow!("Module '{}' not in [modules]", module_name))?;

            // `load_config` already resolves `mod_conf.path` relative to the config file location
            let path = mod_conf.path.clone();

            let code = Editor::new("rust", &source_code, vesper());

            let mut caps = Vec::new();
            for (cap_name, cap_val) in &mod_conf.capabilities {
                // Formats the config as JSON since it's stored as serde_json::Value
                let json_str = serde_json::to_string_pretty(cap_val).unwrap_or_default();
                caps.push(CapConfig {
                    display_name: cap_name.clone(),
                    editor: Editor::new("json", &json_str, vesper()),
                });
            }

            steps.push(ModuleStep {
                name: module_name.clone(),
                path,
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

        let (tx, rx) = std::sync::mpsc::channel();

        Ok(App {
            yaml_path: yaml_path.to_path_buf(),
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
            tx,
            rx,
        })
    }

    fn current_step(&self) -> &ModuleStep { &self.steps[self.selected_step] }
    fn current_step_mut(&mut self) -> &mut ModuleStep { &mut self.steps[self.selected_step] }

    fn save(&mut self) {
        let step = &mut self.steps[self.selected_step];
        if let Some(path) = &step.source_path {
            let content = step.code.get_content();
            match fs::write(path, &content) {
                Ok(_) => {
                    self.status_msg = format!("Saved {}", path.display());
                    step.logs.push(LogLine {
                        text: format!("[save] {} bytes → {}", content.len(), path.display()),
                        level: LogLevel::Info,
                    });
                }
                Err(e) => {
                    self.status_msg = format!("Save failed: {}", e);
                    step.logs.push(LogLine {
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
        let path = self.modules_base_dir.join(&name);
        let src_dir = path.join("src");

        if let Err(e) = fs::create_dir_all(&src_dir) {
            self.status_msg = format!("mkdir failed: {}", e);
            return;
        }

        // Delegate to scaffolding logic from init.rs
        if let Err(e) = crate::init::create_module(&path, &src_dir, &name) {
            self.status_msg = format!("Scaffold failed: {}", e);
            return;
        }

        let lib_path = src_dir.join("lib.rs");
        let lib_rs = fs::read_to_string(&lib_path).unwrap_or_default();
        let code = Editor::new("rust", &lib_rs, vesper());

        self.steps.push(ModuleStep {
            name: name.clone(),
            path,
            code,
            capabilities: vec![],
            logs: vec![LogLine {
                text: format!("── Scaffolded '{}' at {} ──", name, dir.display()),
                level: LogLevel::Info,
            }],
            status: StepStatus::Idle,
        });

        self.selected_step = self.steps.len() - 1;
        self.selected_cap = 0;
        self.status_msg = format!("Created module '{}'", name);
    }

    fn run(&mut self) {
        // Save all modules first
        for step in &self.steps {
            let source = step.path.join("src/lib.rs");
            let _ = fs::write(source, step.code.get_content());
        }

        for step in &mut self.steps {
            step.status = StepStatus::Building;
            step.logs.push(LogLine {
                text: format!("[build] cargo build --target wasm32-unknown-unknown --release -p {} ...", step.name),
                level: LogLevel::Info,
            });
        }

        self.data_rows.clear();
        self.status_msg = "Building and running pipeline...".into();

        let tx = self.tx.clone();
        let yaml_path = self.yaml_path.clone();
        
        let module_paths: Vec<(usize, String, PathBuf)> = self.steps.iter().enumerate()
            .map(|(i, s)| {
                // Determine module base dir dynamically via parents of `src/lib.rs`
                let dir = s.source_path.as_ref()
                    .and_then(|p| p.parent())
                    .and_then(|p| p.parent())
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| PathBuf::from("."));
                (i, s.name.clone(), dir)
            })
            .collect();
            
        let data_path = yaml_path.parent().unwrap_or(Path::new(".")).join("data.jsonl");

        tokio::spawn(async move {
            // 1. Build Wasm Modules via Cargo
            for (idx, name, path) in module_paths {
                let _ = tx.send(TuiEvent::Log(idx, LogLine { text: format!("Building {}...", name), level: LogLevel::Info }));
                
                let output_res = tokio::task::spawn_blocking(move || {
                    std::process::Command::new("cargo")
                        .args(["build", "--target", "wasm32-unknown-unknown", "--release"])
                        .current_dir(&path)
                        .output()
                }).await;
                
                match output_res {
                    Ok(Ok(out)) if out.status.success() => {
                        let _ = tx.send(TuiEvent::Log(idx, LogLine { text: "Build success".into(), level: LogLevel::Info }));
                        let _ = tx.send(TuiEvent::Status(idx, StepStatus::Success));
                    }
                    Ok(Ok(out)) => {
                        let err = String::from_utf8_lossy(&out.stderr);
                        let _ = tx.send(TuiEvent::Log(idx, LogLine { text: format!("Build failed:\n{}", err), level: LogLevel::Error }));
                        let _ = tx.send(TuiEvent::Status(idx, StepStatus::Failed));
                        let _ = tx.send(TuiEvent::RunComplete(Err(anyhow::anyhow!("Build failed for {}", name))));
                        return;
                    }
                    Ok(Err(e)) => {
                        let _ = tx.send(TuiEvent::Log(idx, LogLine { text: format!("Cargo error: {}", e), level: LogLevel::Error }));
                        let _ = tx.send(TuiEvent::Status(idx, StepStatus::Failed));
                        let _ = tx.send(TuiEvent::RunComplete(Err(anyhow::anyhow!("Cargo command error for {}", name))));
                        return;
                    }
                    Err(e) => {
                        let _ = tx.send(TuiEvent::Log(idx, LogLine { text: format!("Tokio join error: {}", e), level: LogLevel::Error }));
                        let _ = tx.send(TuiEvent::Status(idx, StepStatus::Failed));
                        let _ = tx.send(TuiEvent::RunComplete(Err(anyhow::anyhow!("Execution panic for {}", name))));
                        return;
                    }
                }
            }

            // 2. Load Pipeline and Process Data
            let res = async {
                let config = crate::run::load_config(&yaml_path)?;
                let def = pyroduct::pipeline::PipelineDef::load(&config).await?;
                let mut pipeline = pyroduct::pipeline::Pipeline::new(def).await?;
                
                // Fetch input data or use a fallback mock input
                let input_data = if data_path.exists() {
                    fs::read_to_string(&data_path)?
                } else {
                    r#"{"input": "test"}"#.to_string()
                };

                let mut rows = Vec::new();
                for (i, line) in input_data.lines().enumerate() {
                    if line.trim().is_empty() { continue; }
                    
                    let input_row: pyroduct::value::PyroRow<'static> = serde_json::from_str(line)?;
                    let exec = pipeline.process(&input_row).await;
                    
                    // Route module/capability logs back to specific TUI steps
                    for (step_idx, step_exec) in exec.steps.iter().enumerate() {
                        for module_log in &step_exec.logs.module_logs {
                            let _ = tx.send(TuiEvent::Log(step_idx, LogLine { text: module_log.clone(), level: LogLevel::Info }));
                        }
                        for ((lib, cap), cap_logs) in &step_exec.logs.capability_logs {
                            for cap_log in cap_logs {
                                let _ = tx.send(TuiEvent::Log(step_idx, LogLine { text: format!("[{}::{}] {}", lib, cap, cap_log), level: LogLevel::Info }));
                            }
                        }
                    }

                    if let Some(fail) = &exec.failure {
                        let fail_idx = exec.steps.len();
                        for module_log in &fail.logs.module_logs {
                            let _ = tx.send(TuiEvent::Log(fail_idx, LogLine { text: module_log.clone(), level: LogLevel::Info }));
                        }
                        for ((lib, cap), cap_logs) in &fail.logs.capability_logs {
                            for cap_log in cap_logs {
                                let _ = tx.send(TuiEvent::Log(fail_idx, LogLine { text: format!("[{}::{}] {}", lib, cap, cap_log), level: LogLevel::Info }));
                            }
                        }
                        
                        let err_msg = match &fail.result {
                            Ok(e) => format!("{}", e),
                            Err(e) => format!("{}", e),
                        };
                        let _ = tx.send(TuiEvent::Log(fail_idx, LogLine { text: err_msg.clone(), level: LogLevel::Error }));
                        let _ = tx.send(TuiEvent::Status(fail_idx, StepStatus::Failed));
                    }

                    // Convert execution results to our tabular display format
                    let output_str = if let Some(fail) = &exec.failure {
                        match &fail.result {
                            Ok(e) => format!("ERROR: {}", e),
                            Err(e) => format!("ERROR: {}", e),
                        }
                    } else if let Some(last) = exec.steps.last() {
                        last.row.to_string()
                    } else {
                        "No output".to_string()
                    };

                    let status_str = if exec.failure.is_some() { "error".to_string() } else { "ok".to_string() };

                    rows.push(DataRow {
                        columns: vec![
                            ("row".into(), i.to_string()),
                            ("input".into(), line.to_string()),
                            ("output".into(), output_str),
                            ("status".into(), status_str),
                        ]
                    });
                }

                Ok(rows)
            }.await;

            let _ = tx.send(TuiEvent::RunComplete(res));
        });
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
        let editor = Editor::new("json", "{\n  \"key\": \"value\"\n}\n", vesper());
        self.current_step_mut().capabilities.push(CapConfig {
            display_name: format!("cap_{}", n),
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
            } else if val.contains("error") || val.contains("ERROR") {
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
// Main Hook
// =============================================================================

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
        
        // Drain incoming background events
        while let Ok(ev) = app.rx.try_recv() {
            match ev {
                TuiEvent::Log(step, log) => {
                    if step < app.steps.len() {
                        app.steps[step].logs.push(log);
                    }
                }
                TuiEvent::Status(step, st) => {
                    if step < app.steps.len() {
                        app.steps[step].status = st;
                    }
                }
                TuiEvent::RunComplete(res) => {
                    match res {
                        Ok(rows) => {
                            app.data_columns = vec!["row".into(), "input".into(), "output".into(), "status".into()];
                            app.data_rows = rows;
                            app.status_msg = "Pipeline execution complete.".into();
                        }
                        Err(e) => {
                            app.status_msg = format!("Pipeline failed: {}", e);
                        }
                    }
                }
            }
        }

        handle_event(&mut app)?;
        if app.quit { break; }
    }

    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}