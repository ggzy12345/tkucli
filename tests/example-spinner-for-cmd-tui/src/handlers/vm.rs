use tku_core::prelude::*;
use crate::generated::{args::*, handler_traits::VmHandler as VmHandlerTrait};
use crate::shell::{run_streaming, run_with_spinner};

#[derive(Clone, Copy, Default)]
pub struct VmHandler;

#[async_trait]
impl VmHandlerTrait for VmHandler {
    async fn create(&self, ctx: Ctx, args: VmCreateArgs) -> TkucliResult<impl IntoOutput> {
        let spinner = TaskSpinner::start(&ctx, "Launching VM...");
        let (status, _) = run_with_spinner(
            &spinner,
            &format!("multipass launch --name {}", args.name),
        ).await?;
        spinner.stop("");

        if status.success() {
            Ok(Success::new(format!("VM created: {}", args.name)))
        } else {
            Err(anyhow::anyhow!("VM creation failed: {}", args.name).into())
        }
    }

    async fn list(&self, ctx: Ctx, args: VmListArgs) -> TkucliResult<impl IntoOutput> {
        let _ = args.limit;
        let (status, output) = run_streaming(&ctx, "multipass list").await?;

        if status.success() {
            Ok(Success::new(output))
        } else {
            Err(anyhow::anyhow!("VM list failed").into())
        }
    }

    async fn get(&self, ctx: Ctx, args: VmGetArgs) -> TkucliResult<impl IntoOutput> {
        let (status, output) = run_streaming(
            &ctx,
            &format!("multipass info {}", args.name),
        ).await?;

        if status.success() {
            Ok(Success::new(output))
        } else {
            Err(anyhow::anyhow!("VM info failed: {}", args.name).into())
        }
    }

    async fn delete(&self, ctx: Ctx, args: VmDeleteArgs) -> TkucliResult<impl IntoOutput> {
        let spinner = TaskSpinner::start(&ctx, &format!("Deleting VM {}...", args.name));
        let (status, _) = run_with_spinner(
            &spinner,
            &format!("multipass delete {}", args.name),
        ).await?;
        spinner.stop("");

        if status.success() {
            Ok(Success::new(format!("VM deleted: {}", args.name)))
        } else {
            Err(anyhow::anyhow!("VM deletion failed: {}", args.name).into())
        }
    }
}
