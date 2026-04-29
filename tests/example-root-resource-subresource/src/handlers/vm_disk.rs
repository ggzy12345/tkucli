use tku_core::prelude::*;
use crate::generated::{args::*, handler_traits::VmDiskHandler as VmDiskHandlerTrait};

#[derive(Clone, Copy, Default)]
pub struct VmDiskHandler;

#[async_trait]
impl VmDiskHandlerTrait for VmDiskHandler {
    async fn list(&self, _ctx: Ctx, args: VmDiskListArgs) -> TkucliResult<impl IntoOutput> {
        Ok(Success::new(format!("vm disk list called with vm_id={}", args.vm_id)))
    }

    async fn attach(&self, _ctx: Ctx, args: VmDiskAttachArgs) -> TkucliResult<impl IntoOutput> {
        Ok(Success::new(format!(
            "vm disk attach called with vm_id={} disk_id={}",
            args.vm_id, args.disk_id
        )))
    }
}
