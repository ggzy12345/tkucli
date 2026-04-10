use tku_core::prelude::*;
use crate::generated::{args::*, handler_traits::VmHandler as VmHandlerTrait};

#[derive(Clone, Copy, Default)]
pub struct VmHandler;

#[async_trait]
impl VmHandlerTrait for VmHandler {
    async fn list(&self, _ctx: Ctx, args: VmListArgs) -> TkucliResult<impl IntoOutput> {
        Ok(Success::new(format!("vm list called with limit={}", args.limit)))
    }

    async fn get(&self, _ctx: Ctx, args: VmGetArgs) -> TkucliResult<impl IntoOutput> {
        Ok(Success::new(format!("vm get called with id={}", args.id)))
    }
}
