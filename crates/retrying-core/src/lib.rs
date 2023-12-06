extern crate proc_macro;

use proc_macro::TokenStream;

mod code_gen;
mod config;
mod errors;
mod functions;

#[proc_macro_attribute]
pub fn retry(args: TokenStream, item: TokenStream) -> TokenStream {
    functions::prepare_retriable_function(args.into(), item.into()).into()
}
