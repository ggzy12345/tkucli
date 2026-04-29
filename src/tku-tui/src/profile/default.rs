use crate::{
    screen::{ResourceScreen, Screen, ScreenLabels},
    theme::Theme,
};
use tku_core::schema::AppSchema;

pub(super) fn build_initial_screen(
    schema: &AppSchema,
    labels: ScreenLabels,
) -> Box<dyn Screen> {
    ResourceScreen::from_app_schema_with_labels(schema, None, labels)
}

pub(super) fn apply_theme(theme: Theme) -> Theme {
    theme
}
