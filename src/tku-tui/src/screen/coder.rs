use super::{MessageScreen, ResourceScreenState, Screen, ScreenAction, ScreenLabels};
use crate::{events::AppEvent, theme::Theme};
use ratatui::{layout::Rect, Frame};
use tku_core::schema::AppSchema;

/// Copy `text` to the system clipboard using arboard.
/// Returns `Ok(())` on success or an error message.
fn write_clipboard(text: &str) -> Result<(), String> {
    arboard::Clipboard::new()
        .and_then(|mut cb| cb.set_text(text))
        .map_err(|e| e.to_string())
}

pub struct CoderScreen {
    state: ResourceScreenState,
}

impl CoderScreen {
    pub fn from_app_schema(schema: &AppSchema) -> Box<dyn Screen> {
        Self::from_app_schema_with_labels(schema, ScreenLabels::default())
    }

    pub fn from_app_schema_with_labels(
        schema: &AppSchema,
        labels: ScreenLabels,
    ) -> Box<dyn Screen> {
        let state = ResourceScreenState::from_schema(schema, labels);
        if state.resources.is_empty() {
            return MessageScreen::new(
                "No resources",
                "This app has no resources configured in cli.toml.",
            );
        }
        Box::new(Self { state })
    }

    fn current_workspace(&self) -> &str {
        self.state
            .current_resource()
            .map(|r| {
                if r.name == "$root" {
                    "workspace"
                } else {
                    r.name.as_str()
                }
            })
            .unwrap_or("workspace")
    }

    /// Copy the body of the last non-pending transcript entry to the clipboard.
    fn copy_latest_entry(&mut self) {
        let body = self
            .state
            .transcript
            .iter()
            .rev()
            .find(|e| !e.pending)
            .map(|e| e.body.clone());
        match body {
            None => {
                self.state.prompt_message = Some("Nothing to copy".to_string());
            }
            Some(text) => match write_clipboard(&text) {
                Ok(()) => {
                    self.state.prompt_message = Some("Copied to clipboard".to_string());
                }
                Err(e) => {
                    self.state.prompt_message = Some(format!("Copy failed: {e}"));
                }
            },
        }
    }

    /// Copy all transcript entries as plain text to the clipboard.
    fn copy_full_transcript(&mut self) {
        if self.state.transcript.is_empty() {
            self.state.prompt_message = Some("Nothing to copy".to_string());
            return;
        }
        let mut buf = String::new();
        for entry in &self.state.transcript {
            let role = match entry.role {
                super::shared::TranscriptRole::User => "you",
                super::shared::TranscriptRole::Assistant => "coder",
                super::shared::TranscriptRole::System => "system",
            };
            if let Some(title) = &entry.title {
                buf.push_str(&format!("[{role}] {title}\n"));
            } else {
                buf.push_str(&format!("[{role}]\n"));
            }
            buf.push_str(&entry.body);
            buf.push_str("\n\n");
        }
        match write_clipboard(&buf) {
            Ok(()) => {
                self.state.prompt_message = Some("Full conversation copied".to_string());
            }
            Err(e) => {
                self.state.prompt_message = Some(format!("Copy failed: {e}"));
            }
        }
    }

    fn render_transcript(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        use ratatui::{
            layout::Margin,
            style::Style,
            text::{Line, Span, Text},
            widgets::{Block, Paragraph, Wrap},
        };

        frame.render_widget(Block::default().style(Style::default().bg(theme.bg)), area);
        let inner = area;

        self.state.viewport_lines = inner.height;

        let latest_index = self.state.transcript.len().saturating_sub(1);
        let mut lines: Vec<Line<'static>> = Vec::new();
        for (idx, entry) in self.state.transcript.iter().enumerate() {
            let label = match entry.role {
                super::shared::TranscriptRole::User => "› you",
                super::shared::TranscriptRole::Assistant => "• coder",
                super::shared::TranscriptRole::System => "• system",
            };
            let label_style = match entry.role {
                super::shared::TranscriptRole::User => theme.accent_style(),
                super::shared::TranscriptRole::Assistant => Style::default().fg(theme.text),
                super::shared::TranscriptRole::System => theme.dim_style(),
            };

            let mut header = vec![Span::styled(label.to_string(), label_style)];
            if let Some(title) = &entry.title {
                header.push(Span::styled("  ", Style::default()));
                header.push(Span::styled(title.clone(), theme.dim_style()));
            }
            if entry.pending {
                header.push(Span::styled("  ", Style::default()));
                header.push(Span::styled(
                    super::shared::ProgressLabel::short(entry.pending_frame),
                    theme.accent_style(),
                ));
            } else if idx == latest_index {
                header.push(Span::styled("  ", Style::default()));
                header.push(Span::styled(
                    self.state.labels.latest_str().to_string(),
                    theme.dim_style(),
                ));
            }
            lines.push(Line::from(header));

            for body_line in entry.body.lines() {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(body_line.to_string(), Style::default().fg(theme.text)),
                ]));
            }
            lines.push(Line::from(""));
        }

        let paragraph = Paragraph::new(Text::from(lines.clone()))
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(theme.text));

        let wrap_width = inner.width.max(1) as usize;
        self.state.content_lines = lines
            .iter()
            .map(|line| {
                let raw: String = line
                    .spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect();
                let wrapped = textwrap::wrap(&raw, wrap_width);
                wrapped.len().max(1)
            })
            .sum::<usize>()
            .min(u16::MAX as usize) as u16;
        if self.state.auto_follow {
            self.state.scroll_to_bottom();
        }

        frame.render_widget(
            paragraph.scroll((self.state.scroll, 0)),
            inner.inner(Margin {
                horizontal: 0,
                vertical: 0,
            }),
        );
    }

    fn render_status_row(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        use ratatui::{
            style::{Color, Style},
            text::{Line, Span},
            widgets::{Block, Paragraph},
        };

        frame.render_widget(Block::default().style(Style::default().bg(theme.bg)), area);

        if !self.state.has_pending() {
            frame.render_widget(
                Paragraph::new(Line::from(" ")).style(Style::default().bg(theme.bg)),
                area,
            );
            return;
        }

        use super::shared::ProgressLabel;

        let tick = self.state.pending_tick_frame();
        let star = ProgressLabel::star(tick);
        let phrase = format!("{}…", ProgressLabel::phrase(tick));

        let verb = self
            .state
            .current_operation()
            .map(|op| op.verb.as_str())
            .unwrap_or("");

        // ── shimmer: a bright slot bounces left→right→left over `phrase` ──
        let phrase_len = phrase.chars().count();
        // ping-pong: 0..len-1 forward, then len-1..0 backward
        let period = (phrase_len.saturating_sub(1)) * 2;
        let light_pos: f32 = if period == 0 {
            0.0
        } else {
            let m = tick % period.max(1);
            if m < phrase_len {
                m as f32
            } else {
                (period - m) as f32
            }
        };

        let base: (u8, u8, u8) = (120, 110, 100); // dim colour
        let bright: (u8, u8, u8) = (255, 240, 200); // highlight colour
        const WINDOW: f32 = 3.5;

        let shimmer_spans: Vec<Span> = phrase
            .chars()
            .enumerate()
            .map(|(i, ch)| {
                let dist = (i as f32 - light_pos).abs();
                let t = (1.0 - dist / WINDOW).max(0.0).powi(2);
                let r = (base.0 as f32 + (bright.0 as f32 - base.0 as f32) * t) as u8;
                let g = (base.1 as f32 + (bright.1 as f32 - base.1 as f32) * t) as u8;
                let b = (base.2 as f32 + (bright.2 as f32 - base.2 as f32) * t) as u8;
                Span::styled(ch.to_string(), Style::default().fg(Color::Rgb(r, g, b)))
            })
            .collect();

        let suffix = if verb.is_empty() {
            "  ·  esc to stop".to_string()
        } else {
            format!("  ·  {}  ·  esc to stop", verb)
        };

        let mut spans: Vec<Span> = vec![Span::styled(format!("{star} "), theme.accent_style())];
        spans.extend(shimmer_spans);
        spans.push(Span::styled(suffix, theme.dim_style()));

        frame.render_widget(
            Paragraph::new(Line::from(spans)).style(Style::default().bg(theme.bg)),
            area,
        );
    }

    fn render_composer(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        use ratatui::{
            layout::{Alignment, Constraint, Direction, Layout},
            style::Style,
            text::{Line, Span, Text},
            widgets::{Block, Paragraph},
        };

        frame.render_widget(Block::default().style(Style::default().bg(theme.bg)), area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(area);

        let prompt_line = if let Some(composer) = &self.state.composer {
            let caret = if composer.cursor_visible { "▋" } else { " " };
            format!("{} {}{}", self.state.prompt_label(), composer.buffer, caret)
        } else {
            format!("{}", self.state.prompt_label())
        };

        frame.render_widget(
            Paragraph::new(Line::from(" ")).style(Style::default().bg(theme.bg)),
            chunks[0],
        );
        frame.render_widget(
            Paragraph::new(Text::from(vec![Line::from(prompt_line)]))
                .style(Style::default().fg(theme.text).bg(theme.bg)),
            chunks[1],
        );

        if let Some(message) = &self.state.prompt_message {
            frame.render_widget(
                Paragraph::new(Line::from(vec![Span::styled(
                    message.clone(),
                    theme.dim_style(),
                )]))
                .style(Style::default().bg(theme.bg)),
                chunks[2],
            );
        } else {
            let left = format!(
                "{} · {}",
                self.current_workspace(),
                self.state.prompt_label()
            );
            let right = "y copy · Y copy all · Scroll: Ctrl+U/D · PgUp/PgDn";

            let footer_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(0), Constraint::Length(right.len() as u16)])
                .split(chunks[2]);

            frame.render_widget(
                Paragraph::new(Line::from(vec![Span::styled(left, theme.dim_style())]))
                    .style(Style::default().bg(theme.bg)),
                footer_chunks[0],
            );
            frame.render_widget(
                Paragraph::new(Line::from(vec![Span::styled(right, theme.dim_style())]))
                    .style(Style::default().bg(theme.bg))
                    .alignment(Alignment::Right),
                footer_chunks[1],
            );
        }
    }
}

impl Screen for CoderScreen {
    fn title(&self) -> &str {
        "Coder"
    }

    fn shows_status_bar(&self) -> bool {
        false
    }

    fn prefers_inline_results(&self) -> bool {
        true
    }

    fn append_command(&mut self, command: String) {
        self.state.append_command(command);
    }

    fn append_result(&mut self, title: &str, body: String, ok: bool) {
        self.state.append_result(title, body, ok);
    }

    fn begin_pending(&mut self, title: &str, body: String) {
        self.state.begin_pending(title, body);
    }

    fn resolve_pending(&mut self, title: &str, body: String, ok: bool) {
        self.state.resolve_pending(title, body, ok);
    }

    fn update_pending_body(&mut self, msg: &str) {
        self.state.update_pending_body(msg);
    }

    fn handle_event(&mut self, event: &AppEvent) -> ScreenAction {
        use crate::events::is_char;

        // Auto-clear copy feedback on any key press (not tick/mouse).
        if matches!(event, AppEvent::Key(_)) {
            if let Some(msg) = &self.state.prompt_message {
                let is_feedback = msg.starts_with("Copied")
                    || msg.starts_with("Copy failed")
                    || msg.starts_with("Full conversation")
                    || msg.starts_with("Nothing to copy");
                if is_feedback {
                    self.state.prompt_message = None;
                }
            }
        }

        // 'y' — copy latest entry body; 'Y' — copy full transcript.
        // Only intercept when no composer is open (so typing 'y' in input isn't captured).
        if self.state.composer.is_none() {
            if is_char(event, 'y') {
                self.copy_latest_entry();
                return ScreenAction::None;
            }
            if is_char(event, 'Y') {
                self.copy_full_transcript();
                return ScreenAction::None;
            }
        }

        self.state
            .handle_key_event(event)
            .unwrap_or(ScreenAction::None)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        use ratatui::layout::{Constraint, Direction, Layout};

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),
                Constraint::Length(1),
                Constraint::Length(3),
            ])
            .split(area);

        self.render_transcript(frame, chunks[0], theme);
        self.render_status_row(frame, chunks[1], theme);
        self.render_composer(frame, chunks[2], theme);
    }
}
