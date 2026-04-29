use super::{Screen, ScreenAction};
use crate::{events::AppEvent, extension::PaletteItem, theme::Theme};
use ratatui::{layout::Rect, Frame};

pub struct MessageScreen {
    title: String,
    body: String,
}

impl MessageScreen {
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Box<Self> {
        Box::new(Self {
            title: title.into(),
            body: body.into(),
        })
    }
}

impl Screen for MessageScreen {
    fn title(&self) -> &str {
        &self.title
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        use ratatui::{
            layout::{Alignment, Margin},
            text::Text,
            widgets::{Block, Borders, Paragraph, Wrap},
        };

        let block = Block::default()
            .title(self.title.as_str())
            .borders(Borders::ALL)
            .border_style(theme.border_style())
            .title_style(theme.title_style())
            .style(ratatui::style::Style::default().bg(theme.bg));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(
            Paragraph::new(Text::raw(&self.body))
                .wrap(Wrap { trim: true })
                .alignment(Alignment::Left)
                .style(ratatui::style::Style::default().fg(theme.text)),
            inner.inner(Margin {
                horizontal: 1,
                vertical: 1,
            }),
        );
    }

    fn handle_event(&mut self, event: &AppEvent) -> ScreenAction {
        use crate::events::is_key;
        use crossterm::event::KeyCode;

        if is_key(event, KeyCode::Esc) || is_key(event, KeyCode::Enter) {
            ScreenAction::Pop
        } else {
            ScreenAction::None
        }
    }
}

pub struct PaletteScreen {
    items: Vec<PaletteItem>,
    selected: usize,
}

impl PaletteScreen {
    pub fn new(items: Vec<PaletteItem>) -> Box<Self> {
        Box::new(Self { items, selected: 0 })
    }
}

impl Screen for PaletteScreen {
    fn title(&self) -> &str {
        "Palette"
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        use ratatui::{
            layout::Margin,
            style::Style,
            text::{Line, Span, Text},
            widgets::{Block, Borders, Paragraph, Wrap},
        };

        let block = Block::default()
            .title(" Palette ")
            .borders(Borders::ALL)
            .border_style(theme.border_focus_style())
            .title_style(theme.title_style())
            .style(Style::default().bg(theme.bg_secondary));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let lines: Vec<Line> = if self.items.is_empty() {
            vec![Line::from(Span::styled(
                "No palette items registered",
                theme.dim_style(),
            ))]
        } else {
            self.items
                .iter()
                .enumerate()
                .map(|(idx, item)| {
                    let prefix = if idx == self.selected { "▶ " } else { "  " };
                    let style = if idx == self.selected {
                        theme.selected_style()
                    } else {
                        Style::default().fg(theme.text)
                    };
                    let mut spans = vec![
                        Span::styled(prefix, style),
                        Span::styled(item.title.as_str(), style),
                    ];
                    if let Some(desc) = &item.description {
                        spans.push(Span::styled(format!("  {}", desc), theme.dim_style()));
                    }
                    Line::from(spans)
                })
                .collect()
        };

        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .wrap(Wrap { trim: true })
                .style(Style::default().fg(theme.text)),
            inner.inner(Margin {
                horizontal: 1,
                vertical: 1,
            }),
        );
    }

    fn handle_event(&mut self, event: &AppEvent) -> ScreenAction {
        use crate::events::{is_char, is_key};
        use crossterm::event::KeyCode;

        if self.items.is_empty() {
            return if is_key(event, KeyCode::Esc) || is_key(event, KeyCode::Enter) {
                ScreenAction::Pop
            } else {
                ScreenAction::None
            };
        }

        if is_key(event, KeyCode::Down) || is_char(event, 'j') {
            self.selected = (self.selected + 1) % self.items.len();
            ScreenAction::None
        } else if is_key(event, KeyCode::Up) || is_char(event, 'k') {
            self.selected = if self.selected == 0 {
                self.items.len() - 1
            } else {
                self.selected - 1
            };
            ScreenAction::None
        } else if is_key(event, KeyCode::Esc) {
            ScreenAction::Pop
        } else if is_key(event, KeyCode::Enter) {
            let item = &self.items[self.selected];
            ScreenAction::Dispatch {
                resource: item.resource.clone(),
                verb: item.verb.clone(),
                positional: item.positional.clone(),
                flags: item.flags.clone(),
            }
        } else {
            ScreenAction::None
        }
    }
}
