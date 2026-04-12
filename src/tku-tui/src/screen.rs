use crate::{events::AppEvent, extension::PaletteItem, theme::Theme};
use tku_core::schema::{AppSchema, OperationSchema, ResourceSchema};
use ratatui::{layout::Rect, Frame};
use std::collections::HashMap;

// ── ScreenLabels ─────────────────────────────────────────────────────────────

/// Customisable text labels rendered in the transcript bubble header.
///
/// Pass this to [`TuiAppBuilder::labels`] to override the defaults.
///
/// ```rust,ignore
/// TuiApp::builder()
///     // ...
///     .labels(ScreenLabels {
///         running: "working…".to_string(),
///         latest:  "done".to_string(),
///     })
/// ```
#[derive(Clone)]
pub struct ScreenLabels {
    /// Text shown next to the spinner glyph while a command is in-flight.
    /// Default: `"running"`
    pub running: String,
    /// Text shown on the most-recently-completed (non-pending) entry.
    /// Default: `"latest"`
    pub latest: String,
}

impl Default for ScreenLabels {
    fn default() -> Self {
        Self {
            running: "running".to_string(),
            latest:  "latest".to_string(),
        }
    }
}

// ── Screen trait ─────────────────────────────────────────────────────────────

/// Every TUI screen (resource list, detail view, form, dashboard…)
/// implements this trait. The `TuiApp` shell calls `render` and
/// `handle_event` on the currently active screen.
pub trait Screen: Send {
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme);
    fn handle_event(&mut self, event: &AppEvent) -> ScreenAction;
    fn title(&self) -> &str;

    fn prefers_inline_results(&self) -> bool { false }
    fn append_command(&mut self, _command: String) {}
    fn append_result(&mut self, _title: &str, _body: String, _ok: bool) {}
    fn begin_pending(&mut self, _title: &str, _body: String) {}
    fn resolve_pending(&mut self, title: &str, body: String, ok: bool) {
        self.append_result(title, body, ok);
    }
    fn update_pending_body(&mut self, _msg: &str) {}
}

// ── ScreenAction ─────────────────────────────────────────────────────────────

pub enum ScreenAction {
    None,
    Push(Box<dyn Screen>),
    Pop,
    Replace(Box<dyn Screen>),
    Quit,
    Dispatch {
        resource:   String,
        verb:       String,
        positional: Vec<String>,
        flags:      HashMap<String, String>,
    },
}

// ── ResourceScreen ────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ResourceScreen {
    resources:          Vec<TuiResource>,
    selected_resource:  usize,
    selected_operation: usize,
    composer:           Option<ComposerState>,
    prompt_message:     Option<String>,
    transcript:         Vec<TranscriptEntry>,
    scroll:             u16,
    content_lines:      u16,
    viewport_lines:     u16,
    auto_follow:        bool,
    pending_entry:      Option<usize>,
    /// Customisable header labels — set via [`TuiAppBuilder::labels`].
    labels:             ScreenLabels,
}

#[derive(Clone)]
pub struct TuiResource {
    pub name:        String,
    pub description: String,
    pub operations:  Vec<TuiOperation>,
}

#[derive(Clone)]
pub struct TuiOperation {
    pub verb:            String,
    pub description:     String,
    pub positional_args: Vec<String>,
    pub default_flags:   HashMap<String, String>,
    pub required_flags:  Vec<String>,
}

#[derive(Clone, Default)]
struct ComposerState {
    buffer:         String,
    cursor_visible: bool,
}

#[derive(Clone)]
struct TranscriptEntry {
    role:          TranscriptRole,
    title:         Option<String>,
    body:          String,
    pending:       bool,
    pending_frame: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TranscriptRole { User, Assistant, System }

fn welcome_entry() -> TranscriptEntry {
    TranscriptEntry {
        role:  TranscriptRole::System,
        title: Some("tkucli".to_string()),
        body:  "Welcome to Tkucli TUI.\n\n\
                1. Move through actions below with j/k or the arrow keys.\n\
                2. Press Enter to run the selected action.\n\
                3. Results will appear here in the same conversation.\n\
                4. Use Ctrl-U / Ctrl-D or PageUp / PageDown to scroll through history."
            .to_string(),
        pending:       false,
        pending_frame: 0,
    }
}

impl TuiResource {
    pub fn from_schema(resource: &ResourceSchema) -> Self {
        Self::from_schema_path(resource, &[])
    }

    fn from_schema_path(resource: &ResourceSchema, parent_path: &[String]) -> Self {
        let mut path = parent_path.to_vec();
        path.push(resource.name.clone());
        Self {
            name:        path.join("."),
            description: resource.description.clone(),
            operations:  resource.operations.iter().map(TuiOperation::from_schema).collect(),
        }
    }
}

impl TuiOperation {
    fn from_schema(op: &OperationSchema) -> Self {
        let positional_args: Vec<String> =
            op.args.iter().map(|arg| arg.name.clone()).collect();
        let default_flags: HashMap<String, String> = op
            .flags
            .iter()
            .filter_map(|f| f.default.as_ref().map(|v| (f.name.clone(), v.clone())))
            .collect();
        let required_flags: Vec<String> = op
            .flags
            .iter()
            .filter(|f| f.required && f.default.is_none())
            .map(|f| f.name.clone())
            .collect();
        Self {
            verb: op.verb.clone(),
            description: op.description.clone(),
            positional_args,
            default_flags,
            required_flags,
        }
    }
}

impl ResourceScreen {
    pub fn new(resource: TuiResource) -> Self {
        Self {
            resources:          vec![resource],
            selected_resource:  0,
            selected_operation: 0,
            composer:           None,
            prompt_message:     None,
            transcript:         vec![welcome_entry()],
            scroll:             0,
            content_lines:      0,
            viewport_lines:     0,
            auto_follow:        true,
            pending_entry:      None,
            labels:             ScreenLabels::default(),
        }
    }

    /// Override the default header labels. Chains before boxing:
    /// `ResourceScreen::new(r).with_labels(labels)`
    pub fn with_labels(mut self, labels: ScreenLabels) -> Self {
        self.labels = labels;
        self
    }

    pub fn from_resources(resources: Vec<TuiResource>) -> Box<dyn Screen> {
        Self::from_resources_with_labels(resources, ScreenLabels::default())
    }

    pub fn from_resources_with_labels(
        resources: Vec<TuiResource>,
        labels:    ScreenLabels,
    ) -> Box<dyn Screen> {
        Box::new(Self {
            resources,
            selected_resource:  0,
            selected_operation: 0,
            composer:           None,
            prompt_message:     None,
            transcript:         vec![welcome_entry()],
            scroll:             0,
            content_lines:      0,
            viewport_lines:     0,
            auto_follow:        true,
            pending_entry:      None,
            labels,
        })
    }

    pub fn from_schema(resource: &ResourceSchema) -> Box<dyn Screen> {
        Box::new(Self::new(TuiResource::from_schema(resource)))
    }

    pub fn from_app_schema(schema: &AppSchema, resource_name: Option<&str>) -> Box<dyn Screen> {
        Self::from_app_schema_with_labels(schema, resource_name, ScreenLabels::default())
    }

    /// Primary constructor used by `TuiAppBuilder` — accepts custom labels.
    pub fn from_app_schema_with_labels(
        schema:        &AppSchema,
        resource_name: Option<&str>,
        labels:        ScreenLabels,
    ) -> Box<dyn Screen> {
        let mut resources = Vec::new();

        if !schema.root.operations.is_empty() {
            resources.push(TuiResource {
                name:        "$root".to_string(),
                description: "top-level commands".to_string(),
                operations:  schema
                    .root
                    .operations
                    .iter()
                    .map(TuiOperation::from_schema)
                    .collect(),
            });
        }

        for resource in &schema.resources {
            collect_tui_resources(resource, &mut Vec::new(), &mut resources);
        }

        let resources: Vec<TuiResource> = match resource_name {
            Some(name) => resources.into_iter().filter(|r| r.name == name).collect(),
            None       => resources,
        };

        if resources.is_empty() {
            MessageScreen::new(
                "No resources",
                "This app has no resources configured in cli.toml.",
            )
        } else {
            Self::from_resources_with_labels(resources, labels)
        }
    }

    // ── private helpers ───────────────────────────────────────────────────────

    fn current_operation(&self) -> Option<&TuiOperation> {
        self.resources
            .get(self.selected_resource)
            .and_then(|r| r.operations.get(self.selected_operation))
    }

    fn current_resource(&self) -> Option<&TuiResource> {
        self.resources.get(self.selected_resource)
    }

    fn advance_operation(&mut self, delta: isize) {
        let selectable: Vec<(usize, usize)> = self
            .resources
            .iter()
            .enumerate()
            .flat_map(|(ri, r)| r.operations.iter().enumerate().map(move |(oi, _)| (ri, oi)))
            .collect();

        if selectable.is_empty() { return; }

        let current = selectable
            .iter()
            .position(|(ri, oi)| *ri == self.selected_resource && *oi == self.selected_operation)
            .unwrap_or(0);

        let len  = selectable.len() as isize;
        let next = (current as isize + delta).rem_euclid(len) as usize;
        let (ri, oi)            = selectable[next];
        self.selected_resource  = ri;
        self.selected_operation = oi;
        self.composer           = None;
        self.prompt_message     = None;
    }

    fn scroll_by(&mut self, delta: i16) {
        let max_scroll   = self.content_lines.saturating_sub(self.viewport_lines);
        let next         = self.scroll as i32 + delta as i32;
        self.scroll      = next.clamp(0, max_scroll as i32) as u16;
        self.auto_follow = self.scroll >= max_scroll;
    }

    fn scroll_to_bottom(&mut self) {
        self.scroll      = self.content_lines.saturating_sub(self.viewport_lines);
        self.auto_follow = true;
    }

    fn operation_needs_input(&self) -> bool {
        self.current_operation()
            .map(|op| !op.positional_args.is_empty() || !op.required_flags.is_empty())
            .unwrap_or(false)
    }

    fn prompt_label(&self) -> String {
        match (self.current_resource(), self.current_operation()) {
            (Some(r), Some(op)) => {
                if r.name == "$root" { format!("> {}", op.verb) }
                else                 { format!("> {} {}", r.name, op.verb) }
            }
            _ => "> select an action".to_string(),
        }
    }

    fn prompt_placeholder(&self) -> String {
        match self.current_operation() {
            Some(op) => {
                let mut parts = Vec::new();
                for arg  in &op.positional_args { parts.push(format!("<{}>", arg)); }
                for flag in &op.required_flags  { parts.push(format!("{}=<value>", flag)); }
                if parts.is_empty() { "ready".to_string() } else { parts.join(" ") }
            }
            None => "no action selected".to_string(),
        }
    }

    fn build_dispatch(&self, input: &str) -> Result<ScreenAction, String> {
        let resource = self.current_resource()
            .ok_or_else(|| "no resource selected".to_string())?;
        let op = self.current_operation()
            .ok_or_else(|| "no operation selected".to_string())?;

        let mut positional = Vec::new();
        let mut flags      = op.default_flags.clone();

        if !input.trim().is_empty() {
            let tokens: Vec<&str> = input.split_whitespace().collect();
            let mut ti = 0;

            for arg_name in &op.positional_args {
                let token = tokens.get(ti)
                    .ok_or_else(|| format!("missing positional argument `{}`", arg_name))?;
                if token.contains('=') {
                    return Err(format!("expected positional `{}`, got flag-style input", arg_name));
                }
                positional.push((*token).to_string());
                ti += 1;
            }

            let remaining = &tokens[ti..];
            if op.required_flags.len() == 1
                && remaining.len() == 1
                && !remaining[0].contains('=')
            {
                flags.insert(op.required_flags[0].clone(), remaining[0].to_string());
            } else {
                for token in remaining {
                    let (k, v) = token.split_once('=')
                        .ok_or_else(|| format!("expected key=value input, got `{}`", token))?;
                    flags.insert(k.to_string(), v.to_string());
                }
            }
        }

        for flag in &op.required_flags {
            if !flags.contains_key(flag) {
                return Err(format!("missing required flag `{}`", flag));
            }
        }

        Ok(ScreenAction::Dispatch {
            resource:   resource.name.clone(),
            verb:       op.verb.clone(),
            positional,
            flags,
        })
    }

    fn push_bubble_lines(
        lines:     &mut Vec<ratatui::text::Line<'static>>,
        entry:     &TranscriptEntry,
        theme:     &Theme,
        is_latest: bool,
        labels:    &ScreenLabels,
    ) {
        use ratatui::{style::Style, text::{Line, Span}};

        let (label, label_style, body_style, faded_body_style) = match entry.role {
            TranscriptRole::User => (
                "you", theme.selected_style(),
                Style::default().fg(theme.text), theme.dim_style(),
            ),
            TranscriptRole::Assistant => (
                "tkucli", theme.accent_style(),
                Style::default().fg(theme.text), theme.dim_style(),
            ),
            TranscriptRole::System => (
                "system", theme.dim_style(), theme.dim_style(), theme.dim_style(),
            ),
        };

        let active_border_style = if is_latest {
            match entry.role {
                TranscriptRole::User      => theme.selected_style(),
                TranscriptRole::Assistant => theme.accent_style(),
                TranscriptRole::System    => theme.title_style(),
            }
        } else {
            theme.border_style()
        };

        let body_prefix = if is_latest { "▌ " } else { "│ " };

        const SPINNER_FRAMES: [&str; 10] =
            ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

        // Use developer-supplied labels instead of hardcoded strings.
        let status_tag: String = if entry.pending {
            let frame = SPINNER_FRAMES[entry.pending_frame % SPINNER_FRAMES.len()];
            format!("{} {}", frame, labels.running)
        } else if is_latest {
            labels.latest.clone()
        } else {
            String::new()
        };

        let title_style          = if is_latest { Style::default().fg(theme.text_title) } else { theme.dim_style() };
        let rendered_label_style = if is_latest { label_style } else { theme.dim_style() };
        let rendered_body_style  = if is_latest { body_style  } else { faded_body_style  };

        let mut header = vec![
            Span::styled("  ", Style::default()),
            Span::styled("╭ ", active_border_style),
            Span::styled(label.to_string(), rendered_label_style),
        ];
        if let Some(title) = &entry.title {
            header.push(Span::styled("  ", Style::default()));
            header.push(Span::styled(title.clone(), title_style));
        }
        if !status_tag.is_empty() {
            header.push(Span::styled("  ", Style::default()));
            let tag_style = if entry.pending { theme.success_style() } else { theme.accent_style() };
            header.push(Span::styled(status_tag, tag_style));
        }
        lines.push(Line::from(header));

        for body_line in entry.body.lines() {
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(body_prefix, active_border_style),
                Span::styled(body_line.to_string(), rendered_body_style),
            ]));
        }

        lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("╰", active_border_style),
        ]));
        lines.push(Line::from(""));
    }

    fn has_pending(&self) -> bool {
        self.pending_entry
            .and_then(|i| self.transcript.get(i))
            .map(|e| e.pending)
            .unwrap_or(false)
    }
}

fn collect_tui_resources(
    resource:    &ResourceSchema,
    parent_path: &mut Vec<String>,
    out:         &mut Vec<TuiResource>,
) {
    parent_path.push(resource.name.clone());
    out.push(TuiResource::from_schema_path(resource, &parent_path[..parent_path.len() - 1]));
    for child in &resource.subresources {
        collect_tui_resources(child, parent_path, out);
    }
    parent_path.pop();
}

// ── Screen impl for ResourceScreen ───────────────────────────────────────────

impl Screen for ResourceScreen {
    fn title(&self) -> &str { "Tkucli" }
    fn prefers_inline_results(&self) -> bool { true }

    fn append_command(&mut self, command: String) {
        if self.transcript.first()
            .map(|e| e.role == TranscriptRole::System && e.title.as_deref() == Some("tkucli"))
            .unwrap_or(false)
            && self.transcript.len() == 1
        {
            self.transcript.clear();
            self.pending_entry = None;
        }
        self.transcript.push(TranscriptEntry {
            role: TranscriptRole::User, title: None, body: command,
            pending: false, pending_frame: 0,
        });
        if self.auto_follow { self.scroll_to_bottom(); }
    }

    fn append_result(&mut self, title: &str, body: String, ok: bool) {
        let role = if ok { TranscriptRole::Assistant } else { TranscriptRole::System };
        self.transcript.push(TranscriptEntry {
            role, title: Some(title.to_string()), body,
            pending: false, pending_frame: 0,
        });
        if self.auto_follow { self.scroll_to_bottom(); }
    }

    fn begin_pending(&mut self, title: &str, body: String) {
        self.transcript.push(TranscriptEntry {
            role: TranscriptRole::Assistant, title: Some(title.to_string()), body,
            pending: true, pending_frame: 0,
        });
        self.pending_entry = Some(self.transcript.len() - 1);
        if self.auto_follow { self.scroll_to_bottom(); }
    }

    fn resolve_pending(&mut self, title: &str, body: String, ok: bool) {
        let role = if ok { TranscriptRole::Assistant } else { TranscriptRole::System };
        if let Some(index) = self.pending_entry.take() {
            if let Some(entry) = self.transcript.get_mut(index) {
                entry.role          = role;
                entry.title         = Some(title.to_string());
                entry.body          = body;
                entry.pending       = false;
                entry.pending_frame = 0;
                if self.auto_follow { self.scroll_to_bottom(); }
                return;
            }
        }
        self.append_result(title, body, ok);
    }

    fn update_pending_body(&mut self, msg: &str) {
        if let Some(index) = self.pending_entry {
            if let Some(entry) = self.transcript.get_mut(index) {
                if entry.pending {
                    entry.body = msg.to_string();
                    if self.auto_follow { self.scroll_to_bottom(); }
                }
            }
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        use ratatui::{
            layout::{Constraint, Direction, Layout, Margin},
            style::Style,
            text::{Line, Span, Text},
            widgets::{Block, Borders, Paragraph, Wrap},
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),
                Constraint::Min(8),
                Constraint::Length(9),
                Constraint::Length(4),
            ])
            .split(area);
        self.viewport_lines = chunks[1].height.saturating_sub(2);

        let intro = Paragraph::new(Text::from(vec![
            Line::from(vec![
                Span::styled("tkucli", theme.accent_style()),
                Span::styled("  interactive workspace", theme.dim_style()),
            ]),
            Line::from("Pick a resource operation and press Enter to run it."),
            Line::from("Resources live in the main thread so you can browse and act without a sidebar."),
        ]))
        .wrap(Wrap { trim: true })
        .block(Block::default().title(" Session ").borders(Borders::ALL)
            .border_style(theme.border_style()).title_style(theme.title_style())
            .style(Style::default().bg(theme.bg_secondary)))
        .style(Style::default().fg(theme.text));
        frame.render_widget(intro, chunks[0]);

        let transcript_block = Block::default().title(" Conversation ").borders(Borders::ALL)
            .border_style(theme.border_focus_style()).title_style(theme.title_style())
            .style(Style::default().bg(theme.bg));
        let transcript_inner = transcript_block.inner(chunks[1]);
        frame.render_widget(transcript_block, chunks[1]);

        let mut lines: Vec<Line<'static>> = Vec::new();
        let latest_index = self.transcript.len().saturating_sub(1);
        for (idx, entry) in self.transcript.iter().enumerate() {
            Self::push_bubble_lines(&mut lines, entry, theme, idx == latest_index, &self.labels);
        }

        self.content_lines = lines.len().min(u16::MAX as usize) as u16;
        if self.auto_follow { self.scroll_to_bottom(); }

        frame.render_widget(
            Paragraph::new(Text::from(lines)).wrap(Wrap { trim: true })
                .scroll((self.scroll, 0)).style(Style::default().fg(theme.text)),
            transcript_inner.inner(Margin { horizontal: 1, vertical: 1 }),
        );

        let actions_block = Block::default().title(" Actions ").borders(Borders::ALL)
            .border_style(theme.border_style()).title_style(theme.title_style())
            .style(Style::default().bg(theme.bg_secondary));
        let actions_inner = actions_block.inner(chunks[2]);
        frame.render_widget(actions_block, chunks[2]);

        let mut action_lines  = Vec::new();
        let actions_dimmed    = self.has_pending();
        let mut selected_row  = None;

        if self.resources.is_empty() {
            action_lines.push(Line::from(Span::styled("No operations available", theme.dim_style())));
        } else {
            for (resource_idx, resource) in self.resources.iter().enumerate() {
                let display_name = if resource.name == "$root" { "root" } else { resource.name.as_str() };
                action_lines.push(Line::from(vec![
                    Span::styled(display_name,
                        if actions_dimmed { theme.dim_style() } else { Style::default().fg(theme.text_title) }),
                    Span::styled(format!("  {}", resource.description), theme.dim_style()),
                ]));

                if resource.operations.is_empty() {
                    action_lines.push(Line::from(Span::styled("  No operations configured", theme.dim_style())));
                } else {
                    for (operation_idx, op) in resource.operations.iter().enumerate() {
                        let selected = resource_idx == self.selected_resource
                            && operation_idx == self.selected_operation;
                        if selected { selected_row = Some(action_lines.len()); }
                        let prefix     = if selected { "›" } else { " " };
                        let pill_style = if actions_dimmed { theme.dim_style() }
                            else if selected { theme.selected_style() }
                            else { Style::default().bg(theme.bg).fg(theme.text) };
                        action_lines.push(Line::from(vec![
                            Span::styled(format!("{prefix} "),
                                if actions_dimmed { theme.dim_style() } else { Style::default().fg(theme.accent) }),
                            Span::styled(format!(" {} ", op.verb), pill_style),
                            Span::styled(format!("  {}", op.description), theme.dim_style()),
                        ]));
                    }
                }
                action_lines.push(Line::from(""));
            }
        }

        if actions_dimmed {
            action_lines.push(Line::from(Span::styled(
                "Running now. Actions stay available, but the conversation is the focus.",
                theme.dim_style(),
            )));
        }

        let actions_area    = actions_inner.inner(Margin { horizontal: 1, vertical: 1 });
        let viewport_height = actions_area.height as usize;
        let scroll_offset   = selected_row
            .filter(|&row| row >= viewport_height)
            .map(|row| (row - viewport_height / 2) as u16)
            .unwrap_or(0);

        frame.render_widget(
            Paragraph::new(Text::from(action_lines)).wrap(Wrap { trim: true })
                .scroll((scroll_offset, 0)).style(Style::default().fg(theme.text)),
            actions_area,
        );

        let footer = match self.current_operation() {
            Some(_) => {
                let hint = if self.operation_needs_input() { "input" } else { "ready" };
                format!("{}  [{hint}]", self.prompt_label())
            }
            None => "> no actions available".to_string(),
        };

        let prompt_line = if let Some(composer) = &self.composer {
            let caret = if composer.cursor_visible { "▋" } else { " " };
            format!("{} {}{}", self.prompt_label(), composer.buffer, caret)
        } else {
            footer
        };
        let helper_line = if let Some(message) = &self.prompt_message {
            message.clone()
        } else if self.composer.is_some() {
            format!("expected: {}", self.prompt_placeholder())
        } else {
            "j/k move  Enter run/open  Ctrl-U/D scroll  PgUp/PgDn scroll  Ctrl-P palette  q quit".to_string()
        };

        frame.render_widget(
            Paragraph::new(Text::from(vec![
                Line::from(prompt_line),
                Line::from(Span::styled(helper_line, theme.dim_style())),
            ]))
            .wrap(Wrap { trim: true }).style(Style::default().fg(theme.text))
            .block(Block::default().title(" Prompt ").borders(Borders::ALL)
                .border_style(theme.border_focus_style()).title_style(theme.title_style())
                .style(Style::default().bg(theme.bg_secondary))),
            chunks[3],
        );
    }

    fn handle_event(&mut self, event: &AppEvent) -> ScreenAction {
        use crate::events::{is_char, is_key};
        use crossterm::event::{KeyCode, KeyModifiers};

        if matches!(event, AppEvent::Tick) {
            if let Some(composer) = &mut self.composer {
                composer.cursor_visible = !composer.cursor_visible;
            }
            if let Some(index) = self.pending_entry {
                if let Some(entry) = self.transcript.get_mut(index) {
                    entry.pending_frame = entry.pending_frame.wrapping_add(1);
                }
            }
            return ScreenAction::None;
        }

        let has_operations = self.resources.iter().any(|r| !r.operations.is_empty());
        if !has_operations {
            return if is_key(event, KeyCode::Char('q')) || is_key(event, KeyCode::Esc) {
                ScreenAction::Quit
            } else {
                ScreenAction::None
            };
        }

        if let Some(composer) = &mut self.composer {
            if let AppEvent::Key(key) = event {
                match key.code {
                    KeyCode::Esc => {
                        self.composer = None; self.prompt_message = None;
                        return ScreenAction::None;
                    }
                    KeyCode::Backspace => {
                        composer.buffer.pop();
                        composer.cursor_visible = true; self.prompt_message = None;
                        return ScreenAction::None;
                    }
                    KeyCode::Enter => {}
                    KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                        composer.buffer.push(ch);
                        composer.cursor_visible = true; self.prompt_message = None;
                        return ScreenAction::None;
                    }
                    _ => return ScreenAction::None,
                }
                if matches!(key.code, KeyCode::Enter) {
                    let submitted = composer.buffer.trim().to_string();
                    match self.build_dispatch(&submitted) {
                        Ok(action) => { self.composer = None; self.prompt_message = None; return action; }
                        Err(error) => { self.prompt_message = Some(error); return ScreenAction::None; }
                    }
                }
            }
        }

        if is_key(event, KeyCode::Down) || is_char(event, 'j') {
            self.advance_operation(1); ScreenAction::None
        } else if is_key(event, KeyCode::Up) || is_char(event, 'k') {
            self.advance_operation(-1); ScreenAction::None
        } else if is_key(event, KeyCode::PageDown)
            || (is_char(event, 'd') && matches!(event, AppEvent::Key(k) if k.modifiers.contains(KeyModifiers::CONTROL)))
        {
            self.scroll_by((self.viewport_lines.max(1) / 2) as i16); ScreenAction::None
        } else if is_key(event, KeyCode::PageUp)
            || (is_char(event, 'u') && matches!(event, AppEvent::Key(k) if k.modifiers.contains(KeyModifiers::CONTROL)))
        {
            self.scroll_by(-((self.viewport_lines.max(1) / 2) as i16)); ScreenAction::None
        } else if is_key(event, KeyCode::End) {
            self.scroll_to_bottom(); ScreenAction::None
        } else if is_key(event, KeyCode::Home) {
            self.scroll = 0; self.auto_follow = false; ScreenAction::None
        } else if is_key(event, KeyCode::Esc) || is_key(event, KeyCode::Char('q')) {
            ScreenAction::Quit
        } else if is_key(event, KeyCode::Enter) {
            if self.operation_needs_input() {
                self.composer = Some(ComposerState { buffer: String::new(), cursor_visible: true });
                self.prompt_message = Some(format!("expected: {}", self.prompt_placeholder()));
                ScreenAction::None
            } else {
                match self.build_dispatch("") {
                    Ok(action) => action,
                    Err(error) => { self.prompt_message = Some(error); ScreenAction::None }
                }
            }
        } else {
            ScreenAction::None
        }
    }
}

// ── Built-in screens ──────────────────────────────────────────────────────────

pub struct MessageScreen { title: String, body: String }

impl MessageScreen {
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Box<Self> {
        Box::new(Self { title: title.into(), body: body.into() })
    }
}

pub struct PaletteScreen { items: Vec<PaletteItem>, selected: usize }

impl PaletteScreen {
    pub fn new(items: Vec<PaletteItem>) -> Box<Self> {
        Box::new(Self { items, selected: 0 })
    }
}

impl Screen for MessageScreen {
    fn title(&self) -> &str { &self.title }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        use ratatui::{layout::{Alignment, Margin}, text::Text, widgets::{Block, Borders, Paragraph, Wrap}};
        let block = Block::default().title(self.title.as_str()).borders(Borders::ALL)
            .border_style(theme.border_style()).title_style(theme.title_style())
            .style(ratatui::style::Style::default().bg(theme.bg));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(
            Paragraph::new(Text::raw(&self.body)).wrap(Wrap { trim: true })
                .alignment(Alignment::Left)
                .style(ratatui::style::Style::default().fg(theme.text)),
            inner.inner(Margin { horizontal: 1, vertical: 1 }),
        );
    }

    fn handle_event(&mut self, event: &AppEvent) -> ScreenAction {
        use crate::events::is_key;
        use crossterm::event::KeyCode;
        if is_key(event, KeyCode::Esc) || is_key(event, KeyCode::Enter) { ScreenAction::Pop }
        else { ScreenAction::None }
    }
}

impl Screen for PaletteScreen {
    fn title(&self) -> &str { "Palette" }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        use ratatui::{layout::Margin, style::Style, text::{Line, Span, Text}, widgets::{Block, Borders, Paragraph, Wrap}};
        let block = Block::default().title(" Palette ").borders(Borders::ALL)
            .border_style(theme.border_focus_style()).title_style(theme.title_style())
            .style(Style::default().bg(theme.bg_secondary));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let lines: Vec<Line> = if self.items.is_empty() {
            vec![Line::from(Span::styled("No palette items registered", theme.dim_style()))]
        } else {
            self.items.iter().enumerate().map(|(idx, item)| {
                let prefix = if idx == self.selected { "▶ " } else { "  " };
                let style  = if idx == self.selected { theme.selected_style() } else { Style::default().fg(theme.text) };
                let mut spans = vec![Span::styled(prefix, style), Span::styled(item.title.as_str(), style)];
                if let Some(desc) = &item.description {
                    spans.push(Span::styled(format!("  {}", desc), theme.dim_style()));
                }
                Line::from(spans)
            }).collect()
        };

        frame.render_widget(
            Paragraph::new(Text::from(lines)).wrap(Wrap { trim: true }).style(Style::default().fg(theme.text)),
            inner.inner(Margin { horizontal: 1, vertical: 1 }),
        );
    }

    fn handle_event(&mut self, event: &AppEvent) -> ScreenAction {
        use crate::events::{is_char, is_key};
        use crossterm::event::KeyCode;

        if self.items.is_empty() {
            return if is_key(event, KeyCode::Esc) || is_key(event, KeyCode::Enter) {
                ScreenAction::Pop
            } else { ScreenAction::None };
        }

        if is_key(event, KeyCode::Down) || is_char(event, 'j') {
            self.selected = (self.selected + 1) % self.items.len(); ScreenAction::None
        } else if is_key(event, KeyCode::Up) || is_char(event, 'k') {
            self.selected = if self.selected == 0 { self.items.len() - 1 } else { self.selected - 1 };
            ScreenAction::None
        } else if is_key(event, KeyCode::Esc) {
            ScreenAction::Pop
        } else if is_key(event, KeyCode::Enter) {
            let item = &self.items[self.selected];
            ScreenAction::Dispatch {
                resource: item.resource.clone(), verb: item.verb.clone(),
                positional: item.positional.clone(), flags: item.flags.clone(),
            }
        } else { ScreenAction::None }
    }
}
