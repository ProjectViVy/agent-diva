//! Workflows screen: Workflow management with list, runs, and creation wizard.
//!
//! Interaction design 100% copied from AgentDiVA.
//! All data is placeholder/fake for demonstration.

use crate::i18n::Translator;
use crate::theme;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Padding, Paragraph};
use ratatui::Frame;

// ── Data types (placeholder) ──────────────────────────────────────────────────

#[derive(Clone, Default)]
pub struct WorkflowInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub steps: u32,
    pub last_run: Option<String>,
    pub status: String,
}

#[derive(Clone, Default)]
pub struct WorkflowRun {
    pub id: String,
    pub workflow_id: String,
    pub workflow_name: String,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub duration_ms: u64,
    pub steps_completed: u32,
    pub steps_total: u32,
}

// ── State ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WorkflowSubScreen {
    List,
    Runs,
    Create,
    RunInput,
    RunResult,
}

pub struct WorkflowsState {
    pub workflows: Vec<WorkflowInfo>,
    pub runs: Vec<WorkflowRun>,
    pub workflow_list_state: ListState,
    pub run_list_state: ListState,
    pub sub: WorkflowSubScreen,
    pub loading: bool,
    pub tick: usize,
    // Create wizard
    pub create_name: String,
    pub create_desc: String,
    pub create_steps: Vec<String>,
    pub create_step_input: String,
    pub create_field: usize,
    pub create_step_idx: usize,
    // Run input
    pub run_workflow_id: String,
    pub run_inputs: Vec<(String, String)>,
    pub run_input_field: usize,
    // Run result
    pub run_result_status: String,
    pub run_result_output: String,
    // Delete confirmation
    pub show_delete_confirm: bool,
    pub delete_workflow_id: String,
    pub delete_workflow_name: String,
    // Status
    pub status_msg: String,
}

pub enum WorkflowAction {
    Continue,
    Back,
    Refresh,
    CreateWorkflow { name: String, desc: String, steps: Vec<String> },
    RunWorkflow { id: String, inputs: Vec<(String, String)> },
    DeleteWorkflow { id: String },
}

impl WorkflowsState {
    pub fn new() -> Self {
        // Populate with fake data
        let workflows = vec![
            WorkflowInfo {
                id: "wf-001".to_string(),
                name: "code-review".to_string(),
                description: "Review code changes and suggest improvements".to_string(),
                steps: 4,
                last_run: Some("2025-04-15T14:30:00Z".to_string()),
                status: "Active".to_string(),
            },
            WorkflowInfo {
                id: "wf-002".to_string(),
                name: "release-notes".to_string(),
                description: "Generate release notes from git commits".to_string(),
                steps: 3,
                last_run: Some("2025-04-14T09:15:00Z".to_string()),
                status: "Active".to_string(),
            },
            WorkflowInfo {
                id: "wf-003".to_string(),
                name: "test-runner".to_string(),
                description: "Run test suite and report results".to_string(),
                steps: 5,
                last_run: None,
                status: "Draft".to_string(),
            },
            WorkflowInfo {
                id: "wf-004".to_string(),
                name: "deployment".to_string(),
                description: "Deploy to staging environment".to_string(),
                steps: 6,
                last_run: Some("2025-04-10T18:45:00Z".to_string()),
                status: "Paused".to_string(),
            },
        ];

        let runs = vec![
            WorkflowRun {
                id: "run-101".to_string(),
                workflow_id: "wf-001".to_string(),
                workflow_name: "code-review".to_string(),
                status: "Completed".to_string(),
                started_at: "2025-04-15T14:30:00Z".to_string(),
                completed_at: Some("2025-04-15T14:32:15Z".to_string()),
                duration_ms: 135000,
                steps_completed: 4,
                steps_total: 4,
            },
            WorkflowRun {
                id: "run-102".to_string(),
                workflow_id: "wf-002".to_string(),
                workflow_name: "release-notes".to_string(),
                status: "Completed".to_string(),
                started_at: "2025-04-14T09:15:00Z".to_string(),
                completed_at: Some("2025-04-14T09:16:30Z".to_string()),
                duration_ms: 90000,
                steps_completed: 3,
                steps_total: 3,
            },
            WorkflowRun {
                id: "run-103".to_string(),
                workflow_id: "wf-001".to_string(),
                workflow_name: "code-review".to_string(),
                status: "Failed".to_string(),
                started_at: "2025-04-13T11:00:00Z".to_string(),
                completed_at: Some("2025-04-13T11:01:45Z".to_string()),
                duration_ms: 105000,
                steps_completed: 2,
                steps_total: 4,
            },
            WorkflowRun {
                id: "run-104".to_string(),
                workflow_id: "wf-004".to_string(),
                workflow_name: "deployment".to_string(),
                status: "Running".to_string(),
                started_at: "2025-04-16T08:00:00Z".to_string(),
                completed_at: None,
                duration_ms: 0,
                steps_completed: 3,
                steps_total: 6,
            },
        ];

        Self {
            workflows,
            runs,
            workflow_list_state: ListState::default().with_selected(Some(0)),
            run_list_state: ListState::default().with_selected(Some(0)),
            sub: WorkflowSubScreen::List,
            loading: false,
            tick: 0,
            create_name: String::new(),
            create_desc: String::new(),
            create_steps: Vec::new(),
            create_step_input: String::new(),
            create_field: 0,
            create_step_idx: 0,
            run_workflow_id: String::new(),
            run_inputs: Vec::new(),
            run_input_field: 0,
            run_result_status: String::new(),
            run_result_output: String::new(),
            show_delete_confirm: false,
            delete_workflow_id: String::new(),
            delete_workflow_name: String::new(),
            status_msg: String::new(),
        }
    }

    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> WorkflowAction {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return WorkflowAction::Continue;
        }

        // Delete confirmation modal
        if self.show_delete_confirm {
            return self.handle_delete_confirm_key(key);
        }

        match self.sub {
            WorkflowSubScreen::List => self.handle_list_key(key),
            WorkflowSubScreen::Runs => self.handle_runs_key(key),
            WorkflowSubScreen::Create => self.handle_create_key(key),
            WorkflowSubScreen::RunInput => self.handle_run_input_key(key),
            WorkflowSubScreen::RunResult => self.handle_run_result_key(key),
        }
    }

    fn handle_list_key(&mut self, key: KeyEvent) -> WorkflowAction {
        let total = self.workflows.len();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if total > 0 {
                    let i = self.workflow_list_state.selected().unwrap_or(0);
                    let next = if i == 0 { total - 1 } else { i - 1 };
                    self.workflow_list_state.select(Some(next));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if total > 0 {
                    let i = self.workflow_list_state.selected().unwrap_or(0);
                    let next = (i + 1) % total;
                    self.workflow_list_state.select(Some(next));
                }
            }
            KeyCode::Char('n') => {
                self.sub = WorkflowSubScreen::Create;
                self.create_name.clear();
                self.create_desc.clear();
                self.create_steps.clear();
                self.create_step_input.clear();
                self.create_field = 0;
                self.create_step_idx = 0;
            }
            KeyCode::Char('r') => {
                if let Some(idx) = self.workflow_list_state.selected() {
                    if let Some(wf) = self.workflows.get(idx) {
                        self.run_workflow_id = wf.id.clone();
                        // Placeholder inputs
                        self.run_inputs = vec![
                            ("target_branch".to_string(), String::new()),
                            ("review_depth".to_string(), String::new()),
                        ];
                        self.run_input_field = 0;
                        self.sub = WorkflowSubScreen::RunInput;
                    }
                }
            }
            KeyCode::Char('d') => {
                if let Some(idx) = self.workflow_list_state.selected() {
                    if let Some(wf) = self.workflows.get(idx) {
                        self.delete_workflow_id = wf.id.clone();
                        self.delete_workflow_name = wf.name.clone();
                        self.show_delete_confirm = true;
                    }
                }
            }
            KeyCode::Char('2') | KeyCode::Char('R') => {
                self.sub = WorkflowSubScreen::Runs;
            }
            KeyCode::Esc => return WorkflowAction::Back,
            _ => {}
        }
        WorkflowAction::Continue
    }

    fn handle_runs_key(&mut self, key: KeyEvent) -> WorkflowAction {
        let total = self.runs.len();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if total > 0 {
                    let i = self.run_list_state.selected().unwrap_or(0);
                    let next = if i == 0 { total - 1 } else { i - 1 };
                    self.run_list_state.select(Some(next));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if total > 0 {
                    let i = self.run_list_state.selected().unwrap_or(0);
                    let next = (i + 1) % total;
                    self.run_list_state.select(Some(next));
                }
            }
            KeyCode::Char('1') => {
                self.sub = WorkflowSubScreen::List;
            }
            KeyCode::Esc => return WorkflowAction::Back,
            _ => {}
        }
        WorkflowAction::Continue
    }

    fn handle_create_key(&mut self, key: KeyEvent) -> WorkflowAction {
        match key.code {
            KeyCode::Esc => {
                self.sub = WorkflowSubScreen::List;
            }
            KeyCode::Tab => {
                if self.create_field == 2 && !self.create_step_input.is_empty() {
                    self.create_steps.push(self.create_step_input.clone());
                    self.create_step_input.clear();
                }
                self.create_field = (self.create_field + 1) % 3;
            }
            KeyCode::BackTab => {
                self.create_field = if self.create_field == 0 { 2 } else { self.create_field - 1 };
            }
            KeyCode::Enter => {
                if !self.create_name.is_empty() && !self.create_steps.is_empty() {
                    self.sub = WorkflowSubScreen::List;
                    return WorkflowAction::CreateWorkflow {
                        name: self.create_name.clone(),
                        desc: self.create_desc.clone(),
                        steps: self.create_steps.clone(),
                    };
                }
            }
            KeyCode::Char(c) => match self.create_field {
                0 => self.create_name.push(c),
                1 => self.create_desc.push(c),
                2 => self.create_step_input.push(c),
                _ => {}
            },
            KeyCode::Backspace => match self.create_field {
                0 => { self.create_name.pop(); }
                1 => { self.create_desc.pop(); }
                2 => { self.create_step_input.pop(); }
                _ => {}
            },
            _ => {}
        }
        WorkflowAction::Continue
    }

    fn handle_run_input_key(&mut self, key: KeyEvent) -> WorkflowAction {
        match key.code {
            KeyCode::Esc => {
                self.sub = WorkflowSubScreen::List;
            }
            KeyCode::Tab => {
                self.run_input_field = (self.run_input_field + 1) % self.run_inputs.len();
            }
            KeyCode::BackTab => {
                self.run_input_field = if self.run_input_field == 0 {
                    self.run_inputs.len() - 1
                } else {
                    self.run_input_field - 1
                };
            }
            KeyCode::Enter => {
                self.sub = WorkflowSubScreen::RunResult;
                self.run_result_status = "Completed".to_string();
                self.run_result_output = "Workflow executed successfully. All 4 steps completed.\n\nOutput: Generated review report with 12 suggestions.".to_string();
                return WorkflowAction::RunWorkflow {
                    id: self.run_workflow_id.clone(),
                    inputs: self.run_inputs.clone(),
                };
            }
            KeyCode::Char(c) => {
                if let Some((_, val)) = self.run_inputs.get_mut(self.run_input_field) {
                    val.push(c);
                }
            }
            KeyCode::Backspace => {
                if let Some((_, val)) = self.run_inputs.get_mut(self.run_input_field) {
                    val.pop();
                }
            }
            _ => {}
        }
        WorkflowAction::Continue
    }

    fn handle_run_result_key(&mut self, key: KeyEvent) -> WorkflowAction {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                self.sub = WorkflowSubScreen::List;
            }
            _ => {}
        }
        WorkflowAction::Continue
    }

    fn handle_delete_confirm_key(&mut self, key: KeyEvent) -> WorkflowAction {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.show_delete_confirm = false;
                let id = self.delete_workflow_id.clone();
                self.sub = WorkflowSubScreen::List;
                return WorkflowAction::DeleteWorkflow { id };
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.show_delete_confirm = false;
            }
            _ => {}
        }
        WorkflowAction::Continue
    }
}

// ── Drawing ─────────────────────────────────────────────────────────────────

pub fn draw(f: &mut Frame, area: Rect, state: &mut WorkflowsState, _i18n: &Translator) {
    let title = match state.sub {
        WorkflowSubScreen::List => " Workflows ",
        WorkflowSubScreen::Runs => " Workflow Runs ",
        WorkflowSubScreen::Create => " New Workflow ",
        WorkflowSubScreen::RunInput => " Run Workflow ",
        WorkflowSubScreen::RunResult => " Run Result ",
    };

    let block = Block::default()
        .title(Line::from(vec![Span::styled(title, theme::title_style())]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    f.render_widget(block, area);

    match state.sub {
        WorkflowSubScreen::List => draw_list(f, inner, state),
        WorkflowSubScreen::Runs => draw_runs(f, inner, state),
        WorkflowSubScreen::Create => draw_create(f, inner, state),
        WorkflowSubScreen::RunInput => draw_run_input(f, inner, state),
        WorkflowSubScreen::RunResult => draw_run_result(f, inner, state),
    }

    // Delete confirmation modal overlay
    if state.show_delete_confirm {
        draw_delete_confirm(f, area, state);
    }
}

fn draw_list(f: &mut Frame, area: Rect, state: &mut WorkflowsState) {
    let chunks = Layout::vertical([
        Constraint::Length(2), // header + tabs
        Constraint::Min(3),    // list
        Constraint::Length(1), // hints
    ]).split(area);

    // Sub-tabs header
    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled("[1] List", Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled("  ", Style::default()),
                Span::styled("[2] Runs", theme::dim_style()),
            ]),
            Line::from(vec![Span::styled(
                format!("  {:<20} {:<30} {:<8} {:<12} {}",
                    "Name", "Description", "Steps", "Last Run", "Status"),
                theme::table_header(),
            )]),
        ]),
        chunks[0],
    );

    // Workflow list
    if state.loading && state.workflows.is_empty() {
        let spinner = theme::SPINNER_FRAMES[state.tick % theme::SPINNER_FRAMES.len()];
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("  {spinner} "), Style::default().fg(theme::CYAN)),
                Span::styled("Loading workflows...", theme::dim_style()),
            ])),
            chunks[1],
        );
    } else if state.workflows.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(
                "  No workflows. Press [n] to create one.",
                theme::dim_style(),
            )),
            chunks[1],
        );
    } else {
        let items: Vec<ListItem> = state
            .workflows
            .iter()
            .map(|wf| {
                let (status_badge, status_style) = match wf.status.as_str() {
                    "Active" => ("Active", Style::default().fg(theme::GREEN)),
                    "Draft" => ("Draft", Style::default().fg(theme::YELLOW)),
                    "Paused" => ("Paused", Style::default().fg(theme::PURPLE)),
                    _ => (wf.status.as_str(), theme::dim_style()),
                };
                let last_run = wf.last_run.as_ref().map(|t| short_time(t)).unwrap_or_else(|| "Never".to_string());
                ListItem::new(Line::from(vec![
                    Span::styled(format!("  {:<20}", truncate(&wf.name, 19)), Style::default().fg(theme::CYAN)),
                    Span::styled(format!(" {:<30}", truncate(&wf.description, 29)), theme::dim_style()),
                    Span::styled(format!(" {:<8}", wf.steps), Style::default().fg(theme::TEXT)),
                    Span::styled(format!(" {:<12}", last_run), theme::dim_style()),
                    Span::styled(format!(" {}", status_badge), status_style),
                ]))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(theme::selected_style())
            .highlight_symbol("> ");
        f.render_stateful_widget(list, chunks[1], &mut state.workflow_list_state);
    }

    // Hints
    let hints = if !state.status_msg.is_empty() {
        format!("  {} | [n]ew  [r]un  [d]elete  [2]runs  [Esc] back", state.status_msg)
    } else {
        "  [n]ew  [r]un  [d]elete  [2]runs  [Esc] back".to_string()
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(hints, theme::hint_style()))),
        chunks[2],
    );
}

fn draw_runs(f: &mut Frame, area: Rect, state: &mut WorkflowsState) {
    let chunks = Layout::vertical([
        Constraint::Length(2), // header + tabs
        Constraint::Min(3),    // list
        Constraint::Length(1), // hints
    ]).split(area);

    // Sub-tabs header
    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled("[1] List", theme::dim_style()),
                Span::styled("  ", Style::default()),
                Span::styled("[2] Runs", Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![Span::styled(
                format!("  {:<12} {:<16} {:<12} {:<20} {:<12} {}",
                    "Run ID", "Workflow", "Status", "Started", "Duration", "Progress"),
                theme::table_header(),
            )]),
        ]),
        chunks[0],
    );

    // Runs list
    if state.runs.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled("  No workflow runs yet.", theme::dim_style())),
            chunks[1],
        );
    } else {
        let items: Vec<ListItem> = state
            .runs
            .iter()
            .map(|run| {
                let (status_badge, status_style) = match run.status.as_str() {
                    "Completed" => ("Completed", Style::default().fg(theme::GREEN)),
                    "Running" => ("Running", Style::default().fg(theme::CYAN)),
                    "Failed" => ("Failed", Style::default().fg(theme::RED)),
                    "Pending" => ("Pending", Style::default().fg(theme::YELLOW)),
                    _ => (run.status.as_str(), theme::dim_style()),
                };
                let duration = if run.duration_ms > 0 {
                    format!("{:.1}s", run.duration_ms as f64 / 1000.0)
                } else {
                    "N/A".to_string()
                };
                let progress = format!("{}/{}", run.steps_completed, run.steps_total);
                ListItem::new(Line::from(vec![
                    Span::styled(format!("  {:<12}", run.id), Style::default().fg(theme::PURPLE)),
                    Span::styled(format!(" {:<16}", truncate(&run.workflow_name, 15)), Style::default().fg(theme::CYAN)),
                    Span::styled(format!(" {:<12}", status_badge), status_style),
                    Span::styled(format!(" {:<20}", short_time(&run.started_at)), theme::dim_style()),
                    Span::styled(format!(" {:<12}", duration), Style::default().fg(theme::TEXT)),
                    Span::styled(format!(" {}", progress), Style::default().fg(theme::ACCENT)),
                ]))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(theme::selected_style())
            .highlight_symbol("> ");
        f.render_stateful_widget(list, chunks[1], &mut state.run_list_state);
    }

    // Hints
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "  [1]list  [Esc] back",
            theme::hint_style(),
        ))),
        chunks[2],
    );
}

fn draw_create(f: &mut Frame, area: Rect, state: &WorkflowsState) {
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(1),
    ]).split(area);

    let field_style = |idx: usize| {
        if state.create_field == idx {
            Style::default().fg(theme::CYAN).add_modifier(Modifier::BOLD)
        } else {
            theme::dim_style()
        }
    };

    f.render_widget(Paragraph::new(Span::styled("Create New Workflow", Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD))), chunks[0]);

    f.render_widget(Paragraph::new(Span::styled("Name:", field_style(0))), chunks[1]);
    f.render_widget(Paragraph::new(Span::styled(format!("  {}|", state.create_name), Style::default().fg(theme::TEXT))), chunks[2]);

    f.render_widget(Paragraph::new(Span::styled("Description:", field_style(1))), chunks[3]);
    f.render_widget(Paragraph::new(Span::styled(format!("  {}|", state.create_desc), Style::default().fg(theme::TEXT))), chunks[4]);

    f.render_widget(Paragraph::new(Span::styled("Step (press Enter after each, Tab to add):", field_style(2))), chunks[5]);
    f.render_widget(Paragraph::new(Span::styled(format!("  {}|", state.create_step_input), Style::default().fg(theme::TEXT))), chunks[6]);

    // Show added steps
    let steps_text = if state.create_steps.is_empty() {
        "  (no steps added yet)".to_string()
    } else {
        format!("  Steps: {}", state.create_steps.join(" -> "))
    };
    f.render_widget(Paragraph::new(Span::styled(steps_text, theme::dim_style())), chunks[7]);

    f.render_widget(
        Paragraph::new(Span::styled("[Tab] field  [Enter] create  [Esc] cancel", theme::hint_style())),
        chunks[8],
    );
}

fn draw_run_input(f: &mut Frame, area: Rect, state: &WorkflowsState) {
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(5),
        Constraint::Length(1),
    ]).split(area);

    f.render_widget(
        Paragraph::new(Span::styled(
            format!("Run Workflow: {}", state.run_workflow_id),
            Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD),
        )),
        chunks[0],
    );

    // Input fields
    let mut lines = Vec::new();
    for (i, (key, val)) in state.run_inputs.iter().enumerate() {
        let field_style = if state.run_input_field == i {
            Style::default().fg(theme::CYAN).add_modifier(Modifier::BOLD)
        } else {
            theme::dim_style()
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  {}: ", key), field_style),
            Span::styled(format!("{}|", val), Style::default().fg(theme::TEXT)),
        ]));
    }
    f.render_widget(Paragraph::new(lines), chunks[1]);

    f.render_widget(
        Paragraph::new(Span::styled("[Tab] field  [Enter] run  [Esc] cancel", theme::hint_style())),
        chunks[2],
    );
}

fn draw_run_result(f: &mut Frame, area: Rect, state: &WorkflowsState) {
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(1),
    ]).split(area);

    let (status_style, status_icon) = match state.run_result_status.as_str() {
        "Completed" => (Style::default().fg(theme::GREEN), "[OK]"),
        "Failed" => (Style::default().fg(theme::RED), "[ERR]"),
        _ => (Style::default().fg(theme::YELLOW), "[...]"),
    };

    f.render_widget(
        Paragraph::new(Span::styled(
            format!("{} Run {}", status_icon, state.run_result_status),
            status_style.add_modifier(Modifier::BOLD),
        )),
        chunks[0],
    );

    f.render_widget(Paragraph::new(Span::styled("Output:", theme::dim_style())), chunks[1]);

    // Output text
    let output_lines: Vec<Line> = state.run_result_output.lines().map(|l| {
        Line::from(Span::styled(format!("  {}", l), Style::default().fg(theme::TEXT)))
    }).collect();
    f.render_widget(Paragraph::new(output_lines), chunks[2]);

    f.render_widget(
        Paragraph::new(Span::styled("[Enter/Esc] close", theme::hint_style())),
        chunks[3],
    );
}

fn draw_delete_confirm(f: &mut Frame, area: Rect, state: &WorkflowsState) {
    let modal = centered_rect(50, 6, area);
    f.render_widget(Clear, modal);

    let block = Block::default()
        .title(Span::styled(" Confirm Delete ", theme::title_style()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::RED))
        .padding(Padding::uniform(1));
    let inner = block.inner(modal);
    f.render_widget(block, modal);

    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ]).split(inner);

    f.render_widget(
        Paragraph::new(Span::styled(
            format!("Delete workflow '{}'?", state.delete_workflow_name),
            Style::default().fg(theme::TEXT),
        )),
        chunks[0],
    );
    f.render_widget(
        Paragraph::new(Span::styled("This action cannot be undone.", Style::default().fg(theme::YELLOW))),
        chunks[1],
    );
    f.render_widget(
        Paragraph::new(Span::styled("[y] Yes  [n/Esc] No", theme::hint_style())),
        chunks[2],
    );
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}

fn short_time(ts: &str) -> String {
    if let Some(t_pos) = ts.find('T') {
        let time_part = &ts[t_pos + 1..];
        if time_part.len() >= 8 {
            return time_part[..8].to_string();
        }
    }
    ts.chars().take(8).collect()
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let w = area.width * percent_x / 100;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, w, height.min(area.height))
}