extern crate proc_macro;

use crate::config::RetryingConfig;
use proc_macro::TokenStream;
use syn::ItemFn;

mod code_gen;
mod config;
mod errors;

/// macros that allows add retrying functionality to rust functions
/// # Examples
///
/// ```ignore
/// #[retry(stop=(attempts(4)|duration(2)),wait=fixed(1))]
/// fn my_function() -> Result<(),Error> {
/// .....
///}
/// ```
#[proc_macro_attribute]
pub fn retry(args: TokenStream, item: TokenStream) -> TokenStream {
    let config = RetryingConfig::from_token_stream(args.into()).unwrap();

    let function: ItemFn = syn::parse(item).unwrap();

    crate::code_gen::add_retry_code_into_function(function, config).into()
}
