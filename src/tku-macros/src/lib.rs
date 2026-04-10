use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemImpl};

/// Attribute macro applied to a handler `impl` block.
///
/// ```rust,ignore
/// #[tkucli::handler]
/// impl UsersHandler {
///     async fn list(&self, ctx: &Ctx, args: users::ListArgs) -> TkucliResult<impl Render> { … }
/// }
/// ```
///
/// Generates the boilerplate `Handler` trait impl so the struct can be
/// registered with `HandlerRegistry`.
#[proc_macro_attribute]
pub fn handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemImpl);
    let _self_ty = &input.self_ty;

    // Collect method names for informational output; full codegen would
    // iterate over each `ImplItem::Fn` and produce a `Handler` dispatch arm.
    // This stub just re-emits the impl unchanged — the full implementation
    // would emit the `Handler` trait impl alongside it.
    let expanded = quote! {
        #input

        // TODO: emit Handler trait impl for #self_ty
        // This is where the macro would generate:
        //   impl tku_core::handler::Handler for #self_ty { … }
    };

    expanded.into()
}

/// Declarative macro to register multiple handlers and build a `HandlerRegistry`.
///
/// ```rust,ignore
/// let registry = tkucli::register!(UsersHandler, OrdersHandler);
/// ```
#[proc_macro]
pub fn register(input: TokenStream) -> TokenStream {
    let input2 = proc_macro2::TokenStream::from(input);
    let expanded = quote! {
        {
            let mut registry = tku_core::handler::HandlerRegistry::default();
            // Each handler type listed is instantiated and registered.
            // Full implementation iterates the token stream and emits one
            // `registry.register(Box::new(#ident::default()))` per entry.
            let _ = stringify!(#input2); // placeholder
            registry
        }
    };
    expanded.into()
}
