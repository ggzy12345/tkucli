use crate::screen::Screen;
use tku_core::{context::Ctx, schema::AppSchema};
use std::collections::HashMap;

pub struct TuiBuildCtx<'a> {
    pub schema: &'a AppSchema,
    pub ctx:    &'a Ctx,
}

pub trait ScreenFactory: Send + Sync {
    fn id(&self) -> &'static str;

    fn title(&self) -> &'static str {
        self.id()
    }

    fn build(&self, ctx: &TuiBuildCtx<'_>) -> Box<dyn Screen>;
}

pub trait TuiExtension: Send + Sync {
    fn register(&self, registry: &mut TuiRegistry);
}

#[derive(Clone, Default)]
pub struct PaletteItem {
    pub id:          String,
    pub title:       String,
    pub description: Option<String>,
    pub resource:    String,
    pub verb:        String,
    pub positional:  Vec<String>,
    pub flags:       HashMap<String, String>,
}

impl PaletteItem {
    pub fn action(
        id: impl Into<String>,
        title: impl Into<String>,
        resource: impl Into<String>,
        verb: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: None,
            resource: resource.into(),
            verb: verb.into(),
            positional: Vec::new(),
            flags: HashMap::new(),
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn positional(mut self, positional: Vec<String>) -> Self {
        self.positional = positional;
        self
    }

    pub fn flags(mut self, flags: HashMap<String, String>) -> Self {
        self.flags = flags;
        self
    }
}

pub struct TuiRegistry {
    pub(crate) default_screen: Option<String>,
    pub(crate) screens:        Vec<Box<dyn ScreenFactory>>,
    pub(crate) palette_items:  Vec<PaletteItem>,
}

impl TuiRegistry {
    pub fn new() -> Self {
        Self {
            default_screen: None,
            screens: Vec::new(),
            palette_items: Vec::new(),
        }
    }

    pub fn set_default_screen(&mut self, id: impl Into<String>) {
        self.default_screen = Some(id.into());
    }

    pub fn add_screen<F>(&mut self, factory: F)
    where
        F: ScreenFactory + 'static,
    {
        self.screens.push(Box::new(factory));
    }

    pub fn add_palette_item(&mut self, item: PaletteItem) {
        self.palette_items.push(item);
    }
}

impl Default for TuiRegistry {
    fn default() -> Self {
        Self::new()
    }
}
