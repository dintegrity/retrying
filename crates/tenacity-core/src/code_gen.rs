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

    let mut tenacity_variables = quote!(let mut tenacity_retry_attempt=1u32;);
    let tenacity_end_of_loop_cycle = quote!(tenacity_retry_attempt+=1u32;);

    let mut stop_check = quote!();

    match stop {
        Some(StopConfig {
            stop_after_attempt,
            stop_after_duration,
        }) => {
            stop_after_attempt.map(|attempt| {
                stop_check = quote!((tenacity_retry_attempt < #attempt));
            });

            stop_after_duration.map(|duration| {
                tenacity_variables = quote!(
                    #tenacity_variables
                    let tenacity_retry_stop_after_duration_start = std::time::SystemTime::now();
                );
                if !stop_check.is_empty() {
                    stop_check=quote!(#stop_check &&);
                }

                stop_check = quote!(#stop_check (std::time::SystemTime::now().duration_since(tenacity_retry_stop_after_duration_start).unwrap().as_secs() < (#duration as u64)))

            });
        }
        _ => (),
    };

    let mut wait_code = quote!();

    match wait {
        Some(wait_config) => {
            let mut wait_duration_calc = quote!();

            match wait_config {
                WaitConfig::WaitFixed { seconds } => {
                    tenacity_variables = quote!(
                        #tenacity_variables
                        let tenacity_wait_duration=#seconds;
                    )
                }
                WaitConfig::WaitRandom { min, max } => {
                    tenacity_variables = quote!(
                        #tenacity_variables
                        let mut tenacity_wait_duration=0u32;
                        let mut tenacity_wait_random_rng = tenacity::rand::thread_rng();
                    );
                    wait_duration_calc = quote!(
                        tenacity_wait_duration = tenacity_wait_random_rng.gen_range(#min..=#max) as u32;
                    );
                }
                WaitConfig::WaitExponential {
                    multiplier,
                    min,
                    max,
                    exp_base,
                } => {
                    tenacity_variables = quote!(
                        #tenacity_variables
                        let mut tenacity_wait_duration=0u32;
                    );

                    wait_duration_calc = quote!(
                        tenacity_wait_duration = std::cmp::min(#multiplier * #exp_base.pow(tenacity_retry_attempt - 1) + #min , #max);
                    );
                }
                _ => (),
            };

            if asyncness.is_some() {
                unimplemented!()
            } else {
                wait_code = quote!(#wait_duration_calc
                    println!("Sync wait {} seconds", tenacity_wait_duration);
                    std::thread::sleep(std::time::Duration::from_secs(tenacity_wait_duration as u64));
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
        #tenacity_variables

        loop {
            match #block {
                Ok(result) => return Ok(result),
                Err(err) if #stop_check => {
                    println!("New attempt");
                    #wait_code
                },
                Err(err) => break Err(err)
            }
            #tenacity_end_of_loop_cycle
        }
    })
}
