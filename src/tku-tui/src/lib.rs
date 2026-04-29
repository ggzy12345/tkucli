pub mod app;
pub mod events;
pub mod extension;
pub mod profile;
pub mod screen;
pub mod theme;
pub mod widgets;

pub use app::TuiApp;
pub use extension::{PaletteItem, ScreenFactory, TuiBuildCtx, TuiExtension, TuiRegistry};
pub use profile::BuiltinTuiProfile;
pub use screen::{CoderScreen, ResourceScreen, ScreenLabels};
pub use theme::Theme;
