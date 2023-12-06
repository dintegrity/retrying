use crate::config::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemFn, Signature};

pub(crate) fn add_retry_code_into_function(function: ItemFn, config: RetryConfig) -> TokenStream {
    let function_signature = function.sig.clone();

    let ItemFn {
        attrs, vis, block, ..
    } = function;

    let Signature {
        output: return_type,
        inputs: params,
        unsafety,
        asyncness,
        constness,
        abi,
        ident,
        generics:
            syn::Generics {
                params: gen_params,
                where_clause,
                ..
            },
        ..
    } = function_signature;

    let RetryConfig { stop, wait, .. } = config;

    let mut retrying_variables = quote!(let mut retrying_retry_attempts=1u32;);
    let retrying_end_of_loop_cycle = quote!(retrying_retry_attempts+=1u32;);

    let mut stop_check = quote!();

    match stop {
        Some(StopConfig { attempts, duration}) => {
            attempts.map(|a| { stop_check = quote!((retrying_retry_attempts < #a)); });

            duration.map(|duration| {
                retrying_variables = quote!(
                    #retrying_variables
                    let retrying_retry_stop_after_duration_start = std::time::SystemTime::now();
                );
                if !stop_check.is_empty() {
                    stop_check=quote!(#stop_check &&);
                }

                stop_check = quote!(#stop_check (std::time::SystemTime::now().duration_since(retrying_retry_stop_after_duration_start).unwrap().as_secs() < (#duration as u64)))

            });
        }
        _ => (),
    };

    let mut wait_code = quote!();

    match wait {
        Some(wait_config) => {
            let mut wait_duration_calc = quote!();

            match wait_config {
                WaitConfig::Fixed { seconds } => {
                    retrying_variables = quote!(
                        #retrying_variables
                        let retrying_wait_duration=#seconds;
                    )
                }
                WaitConfig::Random { min, max } => {
                    retrying_variables = quote!(
                        #retrying_variables
                        let mut retrying_wait_duration=0u32;
                        use retrying::rand::Rng;
                        let mut retrying_wait_random_rng = retrying::rand::thread_rng();
                    );
                    wait_duration_calc = quote!(
                        retrying_wait_duration = retrying_wait_random_rng.gen_range(#min..=#max) as u32;
                    );
                }
                WaitConfig::Exponential {
                    multiplier,
                    min,
                    max,
                    exp_base,
                } => {
                    retrying_variables = quote!(
                        #retrying_variables
                        let mut retrying_wait_duration=0u32;
                    );

                    wait_duration_calc = quote!(
                        retrying_wait_duration = std::cmp::min(#multiplier * #exp_base.pow(retrying_retry_attempts - 1) + #min , #max);
                    );
                }
                _ => (),
            };

            if asyncness.is_some() {
                unimplemented!()
            } else {
                wait_code = quote!(#wait_duration_calc
                    println!("Sync wait {} seconds", retrying_wait_duration);
                    std::thread::sleep(std::time::Duration::from_secs(retrying_wait_duration as u64));
                );
            }
        }

        None => (),
    }

    quote!(
    #(#attrs) *
    #vis #constness #unsafety #asyncness #abi fn #ident<#gen_params>(#params) #return_type
    #where_clause
    {
        #retrying_variables

        loop {
            match #block {
                Ok(result) => return Ok(result),
                Err(err) if #stop_check => {
                    println!("New attempt");
                    #wait_code
                },
                Err(err) => break Err(err)
            }
            #retrying_end_of_loop_cycle
        }
    })
}
