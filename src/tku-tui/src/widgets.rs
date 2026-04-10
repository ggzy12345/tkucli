use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

/// Sidebar listing all resources. Stateful — tracks which item is selected.
pub struct Sidebar {
    pub items:    Vec<String>,
    pub state:    ListState,
}

impl Sidebar {
    pub fn new(items: Vec<String>) -> Self {
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        Self { items, state }
    }

    pub fn selected(&self) -> Option<&str> {
        self.state.selected().and_then(|i| self.items.get(i)).map(|s| s.as_str())
    }

    pub fn next(&mut self) {
        if self.items.is_empty() { return; }
        let i = self.state.selected().map(|i| (i + 1) % self.items.len()).unwrap_or(0);
        self.state.select(Some(i));
    }

    pub fn prev(&mut self) {
        if self.items.is_empty() { return; }
        let i = self.state.selected().map(|i| {
            if i == 0 { self.items.len() - 1 } else { i - 1 }
        }).unwrap_or(0);
        self.state.select(Some(i));
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, focused: bool) {
        let border_style = if focused {
            theme.border_focus_style()
        } else {
            theme.border_style()
        };

        let block = Block::default()
            .title(" Resources ")
            .borders(Borders::ALL)
            .border_style(border_style)
            .title_style(theme.title_style())
            .style(Style::default().bg(theme.bg_secondary));

        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|name| {
                ListItem::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(name, Style::default().fg(theme.text)),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(theme.selected_style())
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, area, &mut self.state);
    }
}

/// Status bar rendered at the bottom of the screen.
pub struct StatusBar {
    pub message: Option<String>,
}

impl StatusBar {
    pub fn new() -> Self { Self { message: None } }

    pub fn set(&mut self, msg: impl Into<String>) {
        self.message = Some(msg.into());
    }

    pub fn clear(&mut self) {
        self.message = None;
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme, active_screen: &str) {
        let left = Span::styled(
            format!(" tkucli › {active_screen} "),
            theme.accent_style(),
        );
        let right_text = self
            .message
            .as_deref()
            .unwrap_or("  ↑↓ navigate   Enter select   q quit   ? help");
        let right = Span::styled(right_text, theme.dim_style());

        let bar = Paragraph::new(Line::from(vec![left, right]))
            .style(Style::default().bg(theme.bg_secondary));

        frame.render_widget(bar, area);
    }
}

impl Default for StatusBar {
    fn default() -> Self { Self::new() }
}
