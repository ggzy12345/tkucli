use tku_core::prelude::*;
use crate::generated::{args::*, handler_traits::RootHandler as RootHandlerTrait};

#[derive(Clone, Copy, Default)]
pub struct RootHandler;

#[async_trait]
impl RootHandlerTrait for RootHandler {
    async fn list(&self, _ctx: Ctx, args: RootListArgs) -> TkucliResult<impl IntoOutput> {
        Ok(Success::new(format!("list called with limit={}", args.limit)))
    }

    async fn get(&self, _ctx: Ctx, args: RootGetArgs) -> TkucliResult<impl IntoOutput> {
        Ok(Success::new(format!("got id={}", args.id)))
    }
}
