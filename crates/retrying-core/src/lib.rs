extern crate proc_macro;

use proc_macro::TokenStream;

mod code_gen;
mod config;
mod errors;
mod functions;

/// macros that allows add retrying functionality to rust functions
/// # Examples
///
/// ```ignore
/// #[retry(stop=(attempts(4)|duration(2)),wait=fixed(1))]
/// fn my_function() -> Result<i32, core::num::ParseIntError> {
/// "not a i32".parse::<i32>()
///}
/// ```
#[proc_macro_attribute]
pub fn retry(args: TokenStream, item: TokenStream) -> TokenStream {
    functions::prepare_retriable_function(args.into(), item.into()).into()
}
