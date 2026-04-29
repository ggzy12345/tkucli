use crate::{events::AppEvent, theme::Theme};
use ratatui::{layout::Rect, Frame};
use std::collections::HashMap;
use tku_core::schema::{AppSchema, OperationSchema, ResourceSchema};

/// Default values used when the developer leaves a label field unset.
pub(crate) const DEFAULT_LATEST: &str = "latest";
pub(crate) const DEFAULT_WELCOME_TITLE: &str = "tkucli";
pub(crate) const DEFAULT_WELCOME_BODY: &str = "Welcome to Tkucli TUI.\n\n\
    1. Move through actions below with j/k or the arrow keys.\n\
    2. Press Enter to run the selected action.\n\
    3. Results will appear here in the same conversation.\n\
    4. Use Ctrl-U / Ctrl-D or PageUp / PageDown to scroll through history.";

/// Customisable display labels for the TUI shell.
/// Every field is optional — unset fields fall back to the built-in defaults
/// above. Use `..Default::default()` to fill in only what you need:
///
/// ```rust
/// ScreenLabels {
///     welcome_title: Some("My App".to_string()),
///     welcome_body:  Some("Welcome!".to_string()),
///     ..Default::default()          // running / latest keep their defaults
/// }
/// ```
#[derive(Clone, Default)]
pub struct ScreenLabels {
    pub latest: Option<String>,
    pub welcome_title: Option<String>,
    pub welcome_body: Option<String>,
}

impl ScreenLabels {
    /// Returns the `latest` value or the built-in default.
    pub(crate) fn latest_str(&self) -> &str {
        self.latest.as_deref().unwrap_or(DEFAULT_LATEST)
    }
}

pub trait Screen: Send {
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme);
    fn handle_event(&mut self, event: &AppEvent) -> ScreenAction;
    fn title(&self) -> &str;

    fn shows_status_bar(&self) -> bool {
        true
    }
    fn prefers_inline_results(&self) -> bool {
        false
    }
    fn append_command(&mut self, _command: String) {}
    fn append_result(&mut self, _title: &str, _body: String, _ok: bool) {}
    fn begin_pending(&mut self, _title: &str, _body: String) {}
    fn resolve_pending(&mut self, title: &str, body: String, ok: bool) {
        self.append_result(title, body, ok);
    }
    fn update_pending_body(&mut self, _msg: &str) {}
}

pub enum ScreenAction {
    None,
    Push(Box<dyn Screen>),
    Pop,
    Replace(Box<dyn Screen>),
    Quit,
    Dispatch {
        resource: String,
        verb: String,
        positional: Vec<String>,
        flags: HashMap<String, String>,
    },
}

#[derive(Clone)]
pub(crate) struct TuiResource {
    pub name: String,
    pub description: String,
    pub operations: Vec<TuiOperation>,
}

#[derive(Clone)]
pub(crate) struct TuiOperation {
    pub verb: String,
    pub description: String,
    pub positional_args: Vec<String>,
    pub default_flags: HashMap<String, String>,
    pub required_flags: Vec<String>,
}

#[derive(Clone, Default)]
pub(crate) struct ComposerState {
    pub buffer: String,
    pub cursor_visible: bool,
}

#[derive(Clone)]
pub(crate) struct TranscriptEntry {
    pub role: TranscriptRole,
    pub title: Option<String>,
    pub body: String,
    pub pending: bool,
    pub pending_frame: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TranscriptRole {
    User,
    Assistant,
    System,
}

pub(crate) fn welcome_entry(labels: &ScreenLabels) -> TranscriptEntry {
    TranscriptEntry {
        role: TranscriptRole::System,
        title: Some(
            labels
                .welcome_title
                .clone()
                .unwrap_or_else(|| DEFAULT_WELCOME_TITLE.to_string()),
        ),
        body: labels
            .welcome_body
            .clone()
            .unwrap_or_else(|| DEFAULT_WELCOME_BODY.to_string()),
        pending: false,
        pending_frame: 0,
    }
}

pub struct ResourceScreenState {
    pub resources: Vec<TuiResource>,
    pub selected_resource: usize,
    pub selected_operation: usize,
    pub(crate) composer: Option<ComposerState>,
    pub prompt_message: Option<String>,
    pub(crate) transcript: Vec<TranscriptEntry>,
    pub scroll: u16,
    pub content_lines: u16,
    pub viewport_lines: u16,
    pub auto_follow: bool,
    pub pending_entry: Option<usize>,
    pub labels: ScreenLabels,
}

impl ResourceScreenState {
    pub fn new(resources: Vec<TuiResource>, labels: ScreenLabels) -> Self {
        let transcript = vec![welcome_entry(&labels)];
        Self {
            resources,
            selected_resource: 0,
            selected_operation: 0,
            composer: None,
            prompt_message: None,
            transcript,
            scroll: 0,
            content_lines: 0,
            viewport_lines: 0,
            auto_follow: true,
            pending_entry: None,
            labels,
        }
    }

    pub fn from_schema(schema: &AppSchema, labels: ScreenLabels) -> Self {
        let resources = Self::resources_from_schema(schema);
        Self::new(resources, labels)
    }

    fn resources_from_schema(schema: &AppSchema) -> Vec<TuiResource> {
        let mut resources = Vec::new();
        if !schema.root.operations.is_empty() {
            resources.push(TuiResource {
                name: "$root".to_string(),
                description: "top-level commands".to_string(),
                operations: schema
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
        resources
    }

    pub fn advance_operation(&mut self, delta: isize) {
        let selectable: Vec<(usize, usize)> = self
            .resources
            .iter()
            .enumerate()
            .flat_map(|(ri, r)| r.operations.iter().enumerate().map(move |(oi, _)| (ri, oi)))
            .collect();
        if selectable.is_empty() {
            return;
        }
        let current = selectable
            .iter()
            .position(|(ri, oi)| *ri == self.selected_resource && *oi == self.selected_operation)
            .unwrap_or(0);
        let len = selectable.len() as isize;
        let next = (current as isize + delta).rem_euclid(len) as usize;
        let (ri, oi) = selectable[next];
        self.selected_resource = ri;
        self.selected_operation = oi;
        self.composer = None;
        self.prompt_message = None;
    }

    pub fn current_operation(&self) -> Option<&TuiOperation> {
        self.resources
            .get(self.selected_resource)
            .and_then(|r| r.operations.get(self.selected_operation))
    }

    pub fn current_resource(&self) -> Option<&TuiResource> {
        self.resources.get(self.selected_resource)
    }

    pub fn operation_needs_input(&self) -> bool {
        self.current_operation()
            .map(|op| !op.positional_args.is_empty() || !op.required_flags.is_empty())
            .unwrap_or(false)
    }

    pub fn scroll_by(&mut self, delta: i16) {
        let max_scroll = self.content_lines.saturating_sub(self.viewport_lines);
        let next = self.scroll as i32 + delta as i32;
        self.scroll = next.clamp(0, max_scroll as i32) as u16;
        self.auto_follow = self.scroll >= max_scroll;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll = self.content_lines.saturating_sub(self.viewport_lines);
        self.auto_follow = true;
    }

    pub fn append_command(&mut self, command: String) {
        if self
            .transcript
            .first()
            .map(|e| e.role == TranscriptRole::System)
            .unwrap_or(false)
            && self.transcript.len() == 1
        {
            self.transcript.clear();
            self.pending_entry = None;
        }
        self.transcript.push(TranscriptEntry {
            role: TranscriptRole::User,
            title: None,
            body: command,
            pending: false,
            pending_frame: 0,
        });
        if self.auto_follow {
            self.scroll_to_bottom();
        }
    }

    pub fn append_result(&mut self, title: &str, body: String, ok: bool) {
        let role = if ok {
            TranscriptRole::Assistant
        } else {
            TranscriptRole::System
        };
        self.transcript.push(TranscriptEntry {
            role,
            title: Some(title.to_string()),
            body,
            pending: false,
            pending_frame: 0,
        });
        if self.auto_follow {
            self.scroll_to_bottom();
        }
    }

    pub fn begin_pending(&mut self, title: &str, body: String) {
        self.transcript.push(TranscriptEntry {
            role: TranscriptRole::Assistant,
            title: Some(title.to_string()),
            body,
            pending: true,
            pending_frame: 0,
        });
        self.pending_entry = Some(self.transcript.len() - 1);
        if self.auto_follow {
            self.scroll_to_bottom();
        }
    }

    pub fn resolve_pending(&mut self, title: &str, body: String, ok: bool) {
        let role = if ok {
            TranscriptRole::Assistant
        } else {
            TranscriptRole::System
        };
        if let Some(index) = self.pending_entry.take() {
            if let Some(entry) = self.transcript.get_mut(index) {
                entry.role = role;
                entry.title = Some(title.to_string());
                entry.body = body;
                entry.pending = false;
                entry.pending_frame = 0;
                if self.auto_follow {
                    self.scroll_to_bottom();
                }
                return;
            }
        }
        self.append_result(title, body, ok);
    }

    pub fn update_pending_body(&mut self, msg: &str) {
        if let Some(index) = self.pending_entry {
            if let Some(entry) = self.transcript.get_mut(index) {
                if entry.pending {
                    entry.body = msg.to_string();
                    if self.auto_follow {
                        self.scroll_to_bottom();
                    }
                }
            }
        }
    }

    pub fn tick(&mut self) {
        if let Some(composer) = &mut self.composer {
            composer.cursor_visible = !composer.cursor_visible;
        }
        if let Some(index) = self.pending_entry {
            if let Some(entry) = self.transcript.get_mut(index) {
                entry.pending_frame = entry.pending_frame.wrapping_add(1);
            }
        }
    }

    pub fn has_pending(&self) -> bool {
        self.pending_entry
            .and_then(|i| self.transcript.get(i))
            .map(|e| e.pending)
            .unwrap_or(false)
    }

    pub fn prompt_label(&self) -> String {
        match (self.current_resource(), self.current_operation()) {
            (Some(r), Some(op)) => {
                if r.name == "$root" {
                    format!("> {}", op.verb)
                } else {
                    format!("> {} {}", r.name, op.verb)
                }
            }
            _ => "> select an action".to_string(),
        }
    }

    pub fn prompt_placeholder(&self) -> String {
        match self.current_operation() {
            Some(op) => {
                let mut parts = Vec::new();
                for arg in &op.positional_args {
                    parts.push(format!("<{}>", arg));
                }
                for flag in &op.required_flags {
                    parts.push(format!("{}=<value>", flag));
                }
                if parts.is_empty() {
                    "ready".to_string()
                } else {
                    parts.join(" ")
                }
            }
            None => "no action selected".to_string(),
        }
    }

    pub fn build_dispatch(&self, input: &str) -> Result<ScreenAction, String> {
        let resource = self
            .current_resource()
            .ok_or_else(|| "no resource selected".to_string())?;
        let op = self
            .current_operation()
            .ok_or_else(|| "no operation selected".to_string())?;

        let mut positional = Vec::new();
        let mut flags = op.default_flags.clone();

        if !input.trim().is_empty() {
            let tokens: Vec<&str> = input.split_whitespace().collect();
            let mut ti = 0;
            for arg_name in &op.positional_args {
                let token = tokens
                    .get(ti)
                    .ok_or_else(|| format!("missing positional argument `{}`", arg_name))?;
                if token.contains('=') {
                    return Err(format!(
                        "expected positional `{}`, got flag-style input",
                        arg_name
                    ));
                }
                positional.push((*token).to_string());
                ti += 1;
            }
            let remaining = &tokens[ti..];
            if op.required_flags.len() == 1 && remaining.len() == 1 && !remaining[0].contains('=') {
                flags.insert(op.required_flags[0].clone(), remaining[0].to_string());
            } else {
                for token in remaining {
                    let (k, v) = token
                        .split_once('=')
                        .ok_or_else(|| format!("expected key=value, got `{}`", token))?;
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
            resource: resource.name.clone(),
            verb: op.verb.clone(),
            positional,
            flags,
        })
    }

    pub fn handle_key_event(&mut self, event: &AppEvent) -> Option<ScreenAction> {
        use crate::events::{is_char, is_key};
        use crossterm::event::{KeyCode, KeyModifiers, MouseEventKind};

        if matches!(event, AppEvent::Tick) {
            self.tick();
            return Some(ScreenAction::None);
        }

        if let AppEvent::Mouse(mouse) = event {
            match mouse.kind {
                MouseEventKind::ScrollUp => {
                    self.scroll_by(-3);
                    return Some(ScreenAction::None);
                }
                MouseEventKind::ScrollDown => {
                    self.scroll_by(3);
                    return Some(ScreenAction::None);
                }
                _ => {}
            }
        }

        let has_operations = self.resources.iter().any(|r| !r.operations.is_empty());
        if !has_operations {
            return if is_key(event, KeyCode::Char('q')) || is_key(event, KeyCode::Esc) {
                Some(ScreenAction::Quit)
            } else {
                Some(ScreenAction::None)
            };
        }

        if let Some(composer) = &mut self.composer {
            if let AppEvent::Key(key) = event {
                match key.code {
                    KeyCode::Esc => {
                        self.composer = None;
                        self.prompt_message = None;
                        return Some(ScreenAction::None);
                    }
                    KeyCode::Backspace => {
                        composer.buffer.pop();
                        composer.cursor_visible = true;
                        self.prompt_message = None;
                        return Some(ScreenAction::None);
                    }
                    KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                        composer.buffer.push(ch);
                        composer.cursor_visible = true;
                        self.prompt_message = None;
                        return Some(ScreenAction::None);
                    }
                    KeyCode::Enter => {
                        let submitted = self.composer.as_ref().unwrap().buffer.trim().to_string();
                        match self.build_dispatch(&submitted) {
                            Ok(action) => {
                                self.composer = None;
                                self.prompt_message = None;
                                return Some(action);
                            }
                            Err(error) => {
                                self.prompt_message = Some(error);
                                return Some(ScreenAction::None);
                            }
                        }
                    }
                    _ => return Some(ScreenAction::None),
                }
            }
        }

        if is_key(event, KeyCode::Down) || is_char(event, 'j') {
            self.advance_operation(1);
            return Some(ScreenAction::None);
        }
        if is_key(event, KeyCode::Up) || is_char(event, 'k') {
            self.advance_operation(-1);
            return Some(ScreenAction::None);
        }
        if is_key(event, KeyCode::PageDown)
            || (is_char(event, 'd')
                && matches!(event, AppEvent::Key(k) if k.modifiers.contains(KeyModifiers::CONTROL)))
        {
            self.scroll_by((self.viewport_lines.max(1) / 2) as i16);
            return Some(ScreenAction::None);
        }
        if is_key(event, KeyCode::PageUp)
            || (is_char(event, 'u')
                && matches!(event, AppEvent::Key(k) if k.modifiers.contains(KeyModifiers::CONTROL)))
        {
            self.scroll_by(-((self.viewport_lines.max(1) / 2) as i16));
            return Some(ScreenAction::None);
        }
        if is_key(event, KeyCode::End) {
            self.scroll_to_bottom();
            return Some(ScreenAction::None);
        }
        if is_key(event, KeyCode::Home) {
            self.scroll = 0;
            self.auto_follow = false;
            return Some(ScreenAction::None);
        }
        if is_key(event, KeyCode::Esc) || is_key(event, KeyCode::Char('q')) {
            return Some(ScreenAction::Quit);
        }
        if is_key(event, KeyCode::Enter) {
            return Some(if self.operation_needs_input() {
                self.composer = Some(ComposerState {
                    buffer: String::new(),
                    cursor_visible: true,
                });
                self.prompt_message = Some(format!("expected: {}", self.prompt_placeholder()));
                ScreenAction::None
            } else {
                match self.build_dispatch("") {
                    Ok(action) => action,
                    Err(error) => {
                        self.prompt_message = Some(error);
                        ScreenAction::None
                    }
                }
            });
        }

        None
    }

    /// Returns the animation tick frame of the currently pending entry, or 0.
    pub fn pending_tick_frame(&self) -> usize {
        self.pending_entry
            .and_then(|i| self.transcript.get(i))
            .map(|e| e.pending_frame)
            .unwrap_or(0)
    }
}

/// Sparkle/star spinner frames — cycle through these on each tick.
pub(crate) const SPINNER_FRAMES: [&str; 8] = ["✦", "✦", "✧", "⋆", "✧", "✦", "✦", "✧"];

/// Action phrases that cycle slowly while pending — gives the spinner a
/// sense of progress without being distracting.
pub(crate) const WORK_PHRASES: [&str; 6] = [
    "working",
    "thinking",
    "processing",
    "running",
    "computing",
    "almost there",
];

pub struct ProgressLabel;

impl ProgressLabel {
    pub fn star(tick: usize) -> &'static str {
        SPINNER_FRAMES[tick % SPINNER_FRAMES.len()]
    }

    pub fn phrase(tick: usize) -> &'static str {
        // Cycle phrase every 8 ticks (~3.2s at 400ms/tick)
        WORK_PHRASES[(tick / 8) % WORK_PHRASES.len()]
    }

    pub fn full(tick: usize) -> String {
        format!("{} {}…", Self::star(tick), Self::phrase(tick))
    }

    pub fn short(tick: usize) -> String {
        Self::star(tick).to_string()
    }
}

pub(crate) fn push_bubble_lines(
    lines: &mut Vec<ratatui::text::Line<'static>>,
    entry: &TranscriptEntry,
    theme: &Theme,
    is_latest: bool,
    labels: &ScreenLabels,
) {
    use ratatui::{
        style::Style,
        text::{Line, Span},
    };

    let (label, label_style, body_style, faded_body_style) = match entry.role {
        TranscriptRole::User => (
            "you",
            theme.selected_style(),
            Style::default().fg(theme.text),
            theme.dim_style(),
        ),
        TranscriptRole::Assistant => (
            "tkucli",
            theme.accent_style(),
            Style::default().fg(theme.text),
            theme.dim_style(),
        ),
        TranscriptRole::System => (
            "system",
            theme.dim_style(),
            theme.dim_style(),
            theme.dim_style(),
        ),
    };

    let active_border_style = if is_latest {
        match entry.role {
            TranscriptRole::User => theme.selected_style(),
            TranscriptRole::Assistant => theme.accent_style(),
            TranscriptRole::System => theme.title_style(),
        }
    } else {
        theme.border_style()
    };

    let body_prefix = if is_latest { "▌ " } else { "│ " };

    let status_tag: String = if entry.pending {
        ProgressLabel::short(entry.pending_frame)
    } else if is_latest {
        labels.latest_str().to_string()
    } else {
        String::new()
    };

    let title_style = if is_latest {
        ratatui::style::Style::default().fg(theme.text_title)
    } else {
        theme.dim_style()
    };
    let rendered_label_style = if is_latest {
        label_style
    } else {
        theme.dim_style()
    };
    let rendered_body_style = if is_latest {
        body_style
    } else {
        faded_body_style
    };

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
        let tag_style = if entry.pending {
            theme.success_style()
        } else {
            theme.accent_style()
        };
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

impl TuiResource {
    pub fn from_schema(resource: &ResourceSchema) -> Self {
        Self::from_schema_path(resource, &[])
    }

    fn from_schema_path(resource: &ResourceSchema, parent_path: &[String]) -> Self {
        let mut path = parent_path.to_vec();
        path.push(resource.name.clone());
        Self {
            name: path.join("."),
            description: resource.description.clone(),
            operations: resource
                .operations
                .iter()
                .map(TuiOperation::from_schema)
                .collect(),
        }
    }
}

impl TuiOperation {
    fn from_schema(op: &OperationSchema) -> Self {
        let positional_args: Vec<String> = op.args.iter().map(|arg| arg.name.clone()).collect();
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

fn collect_tui_resources(
    resource: &ResourceSchema,
    parent_path: &mut Vec<String>,
    out: &mut Vec<TuiResource>,
) {
    parent_path.push(resource.name.clone());
    out.push(TuiResource::from_schema_path(
        resource,
        &parent_path[..parent_path.len() - 1],
    ));
    for child in &resource.subresources {
        collect_tui_resources(child, parent_path, out);
    }
    parent_path.pop();
}
