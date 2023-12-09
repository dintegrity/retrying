use proc_macro2::TokenStream;
use syn::ItemFn;

use crate::config::RetryingConfig;

pub(crate) fn prepare_retriable_function(args: TokenStream, item: TokenStream) -> TokenStream {
    let config = RetryingConfig::from_token_stream(args).unwrap();

    let function: ItemFn = match syn::parse2(item.clone()) {
        Ok(it) => it,
        Err(e) => panic!("Something wrong {}", e),
    };

    crate::code_gen::add_retry_code_into_function(function, config)
}
