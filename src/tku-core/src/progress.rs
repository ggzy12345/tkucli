use crate::context::Ctx;

/// A unified spinner that abstracts over plain-CLI (cliclack) and TUI
/// (ctx.progress) mode.
///
/// Handlers create one spinner and call `start` / `update` / `stop` without
/// ever branching on `ctx.tui_mode()` themselves — the right back-end is
/// selected automatically.
///
/// # Example
///
/// ```rust,ignore
/// use tku_core::prelude::*;
///
/// async fn my_handler(ctx: Ctx, _args: MyArgs) -> TkucliResult<impl IntoOutput> {
///     let spinner = TaskSpinner::start(&ctx, "Fetching data…");
///     let result  = do_work().await?;
///     spinner.stop("Done.");
///     Ok(Success::new(result))
/// }
/// ```
pub struct TaskSpinner<'a> {
    ctx:     &'a Ctx,
    spinner: Option<cliclack::ProgressBar>,
}

impl<'a> TaskSpinner<'a> {
    /// Start a new spinner with an initial message.
    ///
    /// - In **plain-CLI** mode a `cliclack` spinner is shown in the terminal.
    /// - In **TUI** mode the message is streamed to the TUI progress bubble
    ///   via `ctx.progress`; no terminal spinner is created.
    pub fn start(ctx: &'a Ctx, msg: &str) -> Self {
        let spinner = if !ctx.tui_mode() {
            let s = cliclack::spinner();
            s.start(msg);
            Some(s)
        } else {
            ctx.progress.send(msg);
            None
        };

        Self { ctx, spinner }
    }

    /// Update the current spinner message.
    ///
    /// Always forwards to `ctx.progress` (a no-op in plain-CLI mode when there
    /// is no active TUI receiver), and additionally updates the visible
    /// cliclack spinner text in plain-CLI mode.
    pub fn update(&self, msg: &str) {
        self.ctx.progress.send(msg);

        if let Some(s) = &self.spinner {
            s.set_message(msg);
        }
    }

    /// Stop the spinner and optionally leave a final message in the terminal.
    ///
    /// In TUI mode this is a no-op on the terminal side; the progress bubble
    /// is cleared by the TUI dispatch loop once the handler returns.
    pub fn stop(self, final_msg: &str) {
        if let Some(s) = self.spinner {
            s.stop(final_msg);
        }
    }
}
