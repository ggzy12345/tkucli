use super::{push_bubble_lines, MessageScreen, ResourceScreenState, Screen, ScreenAction, ScreenLabels};
use crate::{events::AppEvent, theme::Theme};
use ratatui::{layout::Rect, Frame};
use tku_core::schema::{AppSchema, ResourceSchema};

pub struct ResourceScreen {
    state: ResourceScreenState,
}

impl ResourceScreen {
    pub fn from_schema(resource: &ResourceSchema) -> Box<dyn Screen> {
        Box::new(Self {
            state: ResourceScreenState::new(
                vec![super::TuiResource::from_schema(resource)],
                ScreenLabels::default(),
            ),
        })
    }

    pub fn from_app_schema(schema: &AppSchema, resource_name: Option<&str>) -> Box<dyn Screen> {
        Self::from_app_schema_with_labels(schema, resource_name, ScreenLabels::default())
    }

    pub fn from_app_schema_with_labels(
        schema: &AppSchema,
        resource_name: Option<&str>,
        labels: ScreenLabels,
    ) -> Box<dyn Screen> {
        let mut state = ResourceScreenState::from_schema(schema, labels);
        if let Some(name) = resource_name {
            state.resources.retain(|r| r.name == name);
        }
        if state.resources.is_empty() {
            return MessageScreen::new("No resources", "This app has no resources configured in cli.toml.");
        }
        Box::new(Self { state })
    }
}

impl Screen for ResourceScreen {
    fn title(&self) -> &str {
        "Tkucli"
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
        self.state.handle_key_event(event).unwrap_or(ScreenAction::None)
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
        self.state.viewport_lines = chunks[1].height.saturating_sub(2);

        let intro = Paragraph::new(Text::from(vec![
            Line::from(vec![
                Span::styled("tkucli", theme.accent_style()),
                Span::styled("  interactive workspace", theme.dim_style()),
            ]),
            Line::from("Pick a resource operation and press Enter to run it."),
            Line::from("Resources live in the main thread so you can browse and act without a sidebar."),
        ]))
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(" Session ")
                .borders(Borders::ALL)
                .border_style(theme.border_style())
                .title_style(theme.title_style())
                .style(Style::default().bg(theme.bg_secondary)),
        )
        .style(Style::default().fg(theme.text));
        frame.render_widget(intro, chunks[0]);

        let transcript_block = Block::default()
            .title(" Conversation ")
            .borders(Borders::ALL)
            .border_style(theme.border_focus_style())
            .title_style(theme.title_style())
            .style(Style::default().bg(theme.bg));
        let transcript_inner = transcript_block.inner(chunks[1]);
        frame.render_widget(transcript_block, chunks[1]);

        let mut lines: Vec<Line<'static>> = Vec::new();
        let latest_index = self.state.transcript.len().saturating_sub(1);
        for (idx, entry) in self.state.transcript.iter().enumerate() {
            push_bubble_lines(&mut lines, entry, theme, idx == latest_index, &self.state.labels);
        }

        self.state.content_lines = lines.len().min(u16::MAX as usize) as u16;
        if self.state.auto_follow {
            self.state.scroll_to_bottom();
        }

        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .wrap(Wrap { trim: true })
                .scroll((self.state.scroll, 0))
                .style(Style::default().fg(theme.text)),
            transcript_inner.inner(Margin {
                horizontal: 1,
                vertical: 1,
            }),
        );

        let actions_block = Block::default()
            .title(" Actions ")
            .borders(Borders::ALL)
            .border_style(theme.border_style())
            .title_style(theme.title_style())
            .style(Style::default().bg(theme.bg_secondary));
        let actions_inner = actions_block.inner(chunks[2]);
        frame.render_widget(actions_block, chunks[2]);

        let mut action_lines = Vec::new();
        let actions_dimmed = self.state.has_pending();
        let mut selected_row = None;

        for (resource_idx, resource) in self.state.resources.iter().enumerate() {
            let display_name = if resource.name == "$root" {
                "root"
            } else {
                resource.name.as_str()
            };
            action_lines.push(Line::from(vec![
                Span::styled(
                    display_name,
                    if actions_dimmed {
                        theme.dim_style()
                    } else {
                        Style::default().fg(theme.text_title)
                    },
                ),
                Span::styled(format!("  {}", resource.description), theme.dim_style()),
            ]));
            for (operation_idx, op) in resource.operations.iter().enumerate() {
                let selected = resource_idx == self.state.selected_resource
                    && operation_idx == self.state.selected_operation;
                if selected {
                    selected_row = Some(action_lines.len());
                }
                let prefix = if selected { "›" } else { " " };
                let pill_style = if actions_dimmed {
                    theme.dim_style()
                } else if selected {
                    theme.selected_style()
                } else {
                    Style::default().bg(theme.bg).fg(theme.text)
                };
                action_lines.push(Line::from(vec![
                    Span::styled(
                        format!("{prefix} "),
                        if actions_dimmed {
                            theme.dim_style()
                        } else {
                            Style::default().fg(theme.accent)
                        },
                    ),
                    Span::styled(format!(" {} ", op.verb), pill_style),
                    Span::styled(format!("  {}", op.description), theme.dim_style()),
                ]));
            }
            action_lines.push(Line::from(""));
        }

        if actions_dimmed {
            action_lines.push(Line::from(Span::styled(
                "Running now. Actions stay available, but the conversation is the focus.",
                theme.dim_style(),
            )));
        }

        let actions_area = actions_inner.inner(Margin {
            horizontal: 1,
            vertical: 1,
        });
        let viewport_height = actions_area.height as usize;
        let scroll_offset = selected_row
            .filter(|&row| row >= viewport_height)
            .map(|row| (row - viewport_height / 2) as u16)
            .unwrap_or(0);

        frame.render_widget(
            Paragraph::new(Text::from(action_lines))
                .wrap(Wrap { trim: true })
                .scroll((scroll_offset, 0))
                .style(Style::default().fg(theme.text)),
            actions_area,
        );

        let prompt_line = if let Some(composer) = &self.state.composer {
            let caret = if composer.cursor_visible { "▋" } else { " " };
            format!("{} {}{}", self.state.prompt_label(), composer.buffer, caret)
        } else {
            let hint = if self.state.operation_needs_input() {
                "input"
            } else {
                "ready"
            };
            format!("{}  [{hint}]", self.state.prompt_label())
        };

        let helper_line = if let Some(message) = &self.state.prompt_message {
            message.clone()
        } else if self.state.composer.is_some() {
            format!("expected: {}", self.state.prompt_placeholder())
        } else {
            "j/k move  Enter run/open  Ctrl-U/D scroll  PgUp/PgDn scroll  Ctrl-P palette  q quit"
                .to_string()
        };

        frame.render_widget(
            Paragraph::new(Text::from(vec![
                Line::from(prompt_line),
                Line::from(Span::styled(helper_line, theme.dim_style())),
            ]))
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(theme.text))
            .block(
                Block::default()
                    .title(" Prompt ")
                    .borders(Borders::ALL)
                    .border_style(theme.border_focus_style())
                    .title_style(theme.title_style())
                    .style(Style::default().bg(theme.bg_secondary)),
            ),
            chunks[3],
        );
    }
}
