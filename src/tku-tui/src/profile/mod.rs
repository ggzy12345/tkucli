mod coder;
mod default;

use crate::{
    screen::{Screen, ScreenLabels},
    theme::Theme,
};
use tku_core::schema::AppSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinTuiProfile {
    Default,
    Coder,
}

impl BuiltinTuiProfile {
    pub fn from_name(name: Option<&str>) -> Option<Self> {
        match name.unwrap_or("default") {
            "default" => Some(Self::Default),
            "coder" => Some(Self::Coder),
            _ => None,
        }
    }

    pub fn build_initial_screen(
        self,
        schema: &AppSchema,
        labels: ScreenLabels,
    ) -> Box<dyn Screen> {
        match self {
            Self::Default => default::build_initial_screen(schema, labels),
            Self::Coder => coder::build_initial_screen(schema, labels),
        }
    }

    pub fn apply_theme(self, theme: Theme) -> Theme {
        match self {
            Self::Default => default::apply_theme(theme),
            Self::Coder => coder::apply_theme(theme),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BuiltinTuiProfile;
    use crate::theme::Theme;
    use ratatui::style::Color;
    #[test]
    fn unknown_profile_name_is_not_builtin() {
        assert_eq!(BuiltinTuiProfile::from_name(Some("custom")), None);
    }

    #[test]
    fn coder_profile_is_recognized() {
        assert_eq!(BuiltinTuiProfile::from_name(Some("coder")), Some(BuiltinTuiProfile::Coder));
    }

    #[test]
    fn coder_profile_respects_light_base_theme() {
        let themed = BuiltinTuiProfile::Coder.apply_theme(Theme::light());
        assert_eq!(themed.bg, Color::Rgb(252, 252, 250));
        assert_eq!(themed.text, Color::Rgb(30, 36, 42));
        assert_eq!(themed.accent, Color::Cyan);
    }
}
