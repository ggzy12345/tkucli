use ratatui::style::{Color, Modifier, Style};

/// Color theme for the TUI shell.
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: &'static str,

    // Chrome
    pub bg:           Color,
    pub bg_secondary: Color,
    pub border:       Color,
    pub border_focus: Color,

    // Text
    pub text:         Color,
    pub text_dim:     Color,
    pub text_title:   Color,

    // Accents
    pub accent:       Color,
    pub accent_dark:  Color,
    pub success:      Color,
    pub warning:      Color,
    pub danger:       Color,

    // Table
    pub row_odd:      Color,
    pub row_even:     Color,
    pub row_selected: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            name:         "dark",
            bg:           Color::Rgb(18, 18, 24),
            bg_secondary: Color::Rgb(28, 28, 36),
            border:       Color::Rgb(50, 50, 65),
            border_focus: Color::Rgb(100, 90, 220),
            text:         Color::Rgb(220, 218, 210),
            text_dim:     Color::Rgb(120, 118, 110),
            text_title:   Color::White,
            accent:       Color::Rgb(100, 90, 220),
            accent_dark:  Color::Rgb(60, 52, 180),
            success:      Color::Rgb(30, 160, 117),
            warning:      Color::Rgb(239, 159, 39),
            danger:       Color::Rgb(216, 90, 48),
            row_odd:      Color::Rgb(22, 22, 30),
            row_even:     Color::Rgb(28, 28, 38),
            row_selected: Color::Rgb(50, 44, 110),
        }
    }

    pub fn light() -> Self {
        Self {
            name:         "light",
            bg:           Color::Rgb(250, 250, 248),
            bg_secondary: Color::Rgb(240, 238, 232),
            border:       Color::Rgb(200, 198, 192),
            border_focus: Color::Rgb(83, 74, 183),
            text:         Color::Rgb(40, 40, 36),
            text_dim:     Color::Rgb(130, 128, 120),
            text_title:   Color::Rgb(20, 20, 18),
            accent:       Color::Rgb(83, 74, 183),
            accent_dark:  Color::Rgb(60, 52, 150),
            success:      Color::Rgb(15, 110, 86),
            warning:      Color::Rgb(186, 117, 23),
            danger:       Color::Rgb(153, 60, 29),
            row_odd:      Color::Rgb(250, 250, 248),
            row_even:     Color::Rgb(243, 241, 235),
            row_selected: Color::Rgb(206, 203, 246),
        }
    }

    pub fn from_name(name: &str) -> Self {
        match name {
            "light" => Self::light(),
            _       => Self::dark(),
        }
    }

    // ── Convenience style builders ────────────────────────────────────────────

    pub fn title_style(&self) -> Style {
        Style::default().fg(self.text_title).add_modifier(Modifier::BOLD)
    }

    pub fn border_style(&self) -> Style {
        Style::default().fg(self.border)
    }

    pub fn border_focus_style(&self) -> Style {
        Style::default().fg(self.border_focus)
    }

    pub fn selected_style(&self) -> Style {
        Style::default()
            .bg(self.row_selected)
            .fg(self.text)
            .add_modifier(Modifier::BOLD)
    }

    pub fn accent_style(&self) -> Style {
        Style::default().fg(self.accent).add_modifier(Modifier::BOLD)
    }

    pub fn dim_style(&self) -> Style {
        Style::default().fg(self.text_dim)
    }

    pub fn success_style(&self) -> Style {
        Style::default().fg(self.success)
    }

    pub fn danger_style(&self) -> Style {
        Style::default().fg(self.danger)
    }
}
