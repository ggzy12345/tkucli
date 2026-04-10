use crate::{
    extension::{PaletteItem, ScreenFactory, TuiBuildCtx, TuiExtension, TuiRegistry},
    events::{spawn_event_loop, AppEvent},
    screen::{PaletteScreen, ResourceScreen, Screen, ScreenAction},
    theme::Theme,
    widgets::StatusBar,
};
use crossterm::{
    event::{KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    Terminal,
};
use tku_core::{
    context::Ctx,
    extract::{ArgValue, ParsedArgs},
    handler::{CliRequest, CliService},
    schema::AppSchema,
};
use std::{collections::HashMap, io, sync::Arc};

/// The top-level TUI shell. Holds the screen stack, sidebar, and
/// status bar. Drives the ratatui event loop.
pub struct TuiApp {
    theme:       Theme,
    status_bar:  StatusBar,
    _custom_screens: HashMap<String, Arc<dyn ScreenFactory>>,
    palette_items:  Vec<PaletteItem>,
    service:     Arc<dyn CliService>,
    ctx:         Ctx,
    /// Stack of screens; the last element is the active one.
    screen_stack: Vec<Box<dyn Screen>>,
}

pub struct TuiAppBuilder {
    theme:      Option<Theme>,
    schema:     Option<AppSchema>,
    service:    Option<Arc<dyn CliService>>,
    ctx:        Option<Ctx>,
    extensions: Vec<Box<dyn TuiExtension>>,
}

impl TuiApp {
    pub fn new(
        theme: Theme,
        initial_screen: Box<dyn Screen>,
        custom_screens: HashMap<String, Arc<dyn ScreenFactory>>,
        palette_items: Vec<PaletteItem>,
        service: Arc<dyn CliService>,
        ctx: Ctx,
    ) -> Self {
        Self {
            theme,
            status_bar: StatusBar::new(),
            _custom_screens: custom_screens,
            palette_items,
            service,
            ctx,
            screen_stack: vec![initial_screen],
        }
    }

    pub fn builder() -> TuiAppBuilder {
        TuiAppBuilder {
            theme: None,
            schema: None,
            service: None,
            ctx: None,
            extensions: Vec::new(),
        }
    }

    pub fn from_schema(
        theme: Theme,
        schema: &AppSchema,
        service: Arc<dyn CliService>,
        ctx: Ctx,
    ) -> Self {
        Self::builder()
            .theme(theme)
            .schema(schema.clone())
            .service(service)
            .ctx(ctx)
            .build()
            .expect("TuiApp::from_schema requires a complete builder")
    }

    /// Run the TUI event loop. Blocks until the user quits.
    pub async fn run(mut self) -> anyhow::Result<()> {
        // Setup terminal.
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend  = CrosstermBackend::new(stdout);
        let mut term = Terminal::new(backend)?;

        let mut events = spawn_event_loop(400);

        loop {
            // Draw.
            term.draw(|frame| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(1), Constraint::Length(1)])
                    .split(frame.size());

                // Active screen
                if let Some(screen) = self.screen_stack.last_mut() {
                    screen.render(frame, chunks[0], &self.theme);
                    let title = screen.title().to_owned();
                    self.status_bar.render(frame, chunks[1], &self.theme, &title);
                }
            })?;

            // Handle next event.
            let Some(event) = events.recv().await else { break };

            match &event {
                AppEvent::Quit => break,
                AppEvent::Key(k)
                    if k.code == KeyCode::Char('p')
                        && k.modifiers.contains(KeyModifiers::CONTROL) =>
                {
                    if !self.palette_items.is_empty() {
                        self.screen_stack.push(PaletteScreen::new(self.palette_items.clone()));
                    } else {
                        self.status_bar.set("No palette items registered");
                    }
                    continue;
                }
                _ => {}
            }

            match self.screen_stack.last_mut() {
                Some(screen) => {
                    let action = screen.handle_event(&event);
                    if self.apply_action(action).await { break; }
                }
                None => break,
            }
        }

        // Restore terminal.
        disable_raw_mode()?;
        execute!(term.backend_mut(), LeaveAlternateScreen)?;
        term.show_cursor()?;
        Ok(())
    }

    /// Returns `true` if the app should quit.
    async fn apply_action(&mut self, action: ScreenAction) -> bool {
        match action {
            ScreenAction::None => false,
            ScreenAction::Quit => true,
            ScreenAction::Pop  => {
                if self.screen_stack.len() > 1 {
                    self.screen_stack.pop();
                }
                false
            }
            ScreenAction::Push(screen) => {
                self.screen_stack.push(screen);
                false
            }
            ScreenAction::Replace(screen) => {
                self.screen_stack.pop();
                self.screen_stack.push(screen);
                false
            }
            ScreenAction::Dispatch { resource, verb, positional, flags } => {
                if self
                    .screen_stack
                    .last()
                    .map(|screen| !screen.prefers_inline_results())
                    .unwrap_or(false)
                    && self.screen_stack.len() > 1
                {
                    self.screen_stack.pop();
                }

                let command = format_command(&resource, &verb, &positional, &flags);
                // For root operations hide the sentinel "$root" from user-visible labels.
                let display_label = if resource == "$root" {
                    verb.clone()
                } else {
                    format!("{resource} {verb}")
                };
                if let Some(screen) = self.screen_stack.last_mut() {
                    screen.append_command(command);
                    screen.begin_pending(
                        &display_label,
                        "Running command...".to_string(),
                    );
                }

                self.status_bar.set(format!("Running {display_label}…"));
                let mut args = ParsedArgs::new();
                for value in positional {
                    args.push(value);
                }
                for (key, value) in flags {
                    args.insert(key, ArgValue::String(value));
                }

                let req = CliRequest::new(self.ctx.clone(), resource.clone(), verb.clone(), args);
                match self.service.call(req).await {
                    Ok(output) => {
                        self.status_bar.set(format!("Completed {display_label}"));
                        let rendered = output.render(self.ctx.format());
                        if let Some(screen) = self.screen_stack.last_mut() {
                            screen.resolve_pending(&display_label, rendered, true);
                        }
                    }
                    Err(error) => {
                        self.status_bar.set(format!("Failed {display_label}"));
                        if let Some(screen) = self.screen_stack.last_mut() {
                            screen.resolve_pending(
                                &display_label,
                                format!("Error: {error}"),
                                false,
                            );
                        }
                    }
                }
                false
            }
        }
    }
}

fn format_command(
    resource: &str,
    verb: &str,
    positional: &[String],
    flags: &HashMap<String, String>,
) -> String {
    // For root operations omit the internal "$root" sentinel so the history shows just the verb.
    let mut parts = if resource == "$root" {
        vec![verb.to_string()]
    } else {
        vec![resource.to_string(), verb.to_string()]
    };
    parts.extend(positional.iter().cloned());

    let mut flag_parts: Vec<String> = flags
        .iter()
        .map(|(key, value)| format!("--{key} {value}"))
        .collect();
    flag_parts.sort();
    parts.extend(flag_parts);

    parts.join(" ")
}

impl TuiAppBuilder {
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    pub fn schema(mut self, schema: AppSchema) -> Self {
        self.schema = Some(schema);
        self
    }

    pub fn service(mut self, service: Arc<dyn CliService>) -> Self {
        self.service = Some(service);
        self
    }

    pub fn ctx(mut self, ctx: Ctx) -> Self {
        self.ctx = Some(ctx);
        self
    }

    pub fn extension<E>(mut self, extension: E) -> Self
    where
        E: TuiExtension + 'static,
    {
        self.extensions.push(Box::new(extension));
        self
    }

    pub fn build(self) -> anyhow::Result<TuiApp> {
        let theme = self.theme.ok_or_else(|| anyhow::anyhow!("missing theme"))?;
        let schema = self.schema.ok_or_else(|| anyhow::anyhow!("missing schema"))?;
        let service = self.service.ok_or_else(|| anyhow::anyhow!("missing service"))?;
        let ctx = self.ctx.ok_or_else(|| anyhow::anyhow!("missing context"))?;

        let mut registry = TuiRegistry::new();
        for extension in self.extensions {
            extension.register(&mut registry);
        }

        let build_ctx = TuiBuildCtx { schema: &schema, ctx: &ctx };
        let mut custom_screens: HashMap<String, Arc<dyn ScreenFactory>> = HashMap::new();
        for screen in registry.screens {
            custom_screens.insert(screen.id().to_string(), Arc::from(screen));
        }

        let default_screen = registry
            .default_screen
            .clone()
            .or_else(|| schema.tui.default_screen.clone());

        let initial_screen = if let Some(default_screen) = default_screen.as_deref() {
            match custom_screens.get(default_screen) {
                Some(factory) => factory.build(&build_ctx),
                None => ResourceScreen::from_app_schema(&schema, Some(default_screen)),
            }
        } else {
            ResourceScreen::from_app_schema(&schema, None)
        };

        Ok(TuiApp::new(
            theme,
            initial_screen,
            custom_screens,
            registry.palette_items,
            service,
            ctx,
        ))
    }
}
