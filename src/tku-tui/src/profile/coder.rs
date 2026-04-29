use crate::{
    screen::{CoderScreen, Screen, ScreenLabels},
    theme::Theme,
};
use ratatui::style::Color;
use tku_core::schema::AppSchema;

pub(super) fn build_initial_screen(
    schema: &AppSchema,
    labels: ScreenLabels,
) -> Box<dyn Screen> {
    CoderScreen::from_app_schema_with_labels(schema, labels)
}

pub(super) fn apply_theme(mut theme: Theme) -> Theme {
    theme.border_focus = Color::Cyan;
    theme.accent = Color::Cyan;
    theme.success = Color::Green;
    theme.danger = Color::Red;

    if theme.name == "light" {
        theme.border = Color::Rgb(190, 198, 205);
        theme.text = Color::Rgb(30, 36, 42);
        theme.text_dim = Color::Rgb(110, 118, 126);
        theme.text_title = Color::Rgb(20, 26, 32);
        theme.accent_dark = Color::Rgb(55, 120, 140);
        theme.row_selected = Color::Rgb(214, 238, 244);
        theme.bg = Color::Rgb(252, 252, 250);
        theme.bg_secondary = Color::Rgb(252, 252, 250);
        theme.row_odd = theme.bg;
        theme.row_even = theme.bg;
    } else {
        theme.border = Color::Rgb(40, 44, 52);
        theme.text = Color::Rgb(232, 236, 241);
        theme.text_dim = Color::Rgb(135, 141, 150);
        theme.text_title = Color::Rgb(240, 244, 248);
        theme.accent_dark = Color::Rgb(56, 124, 138);
        theme.row_selected = Color::Rgb(24, 46, 54);
        theme.bg = Color::Rgb(13, 15, 19);
        theme.bg_secondary = Color::Rgb(13, 15, 19);
        theme.row_odd = theme.bg;
        theme.row_even = theme.bg;
    }

    theme
}
