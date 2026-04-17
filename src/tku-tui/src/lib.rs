pub mod app;
pub mod events;
pub mod extension;
pub mod screen;
pub mod theme;
pub mod widgets;

pub use app::TuiApp;
pub use extension::{PaletteItem, ScreenFactory, TuiBuildCtx, TuiExtension, TuiRegistry};
pub use theme::Theme;
