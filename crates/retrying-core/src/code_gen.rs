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

    let RetryConfig { stop, wait, envs_prefix, .. } = config;

    let mut retrying_variables = quote!(let mut retrying_retry_attempt=1u32;);
    let retrying_end_of_loop_cycle = quote!(retrying_retry_attempt+=1u32;);

    let mut stop_check = quote!(true);

    match stop {
        Some(StopConfig { attempts, duration}) => {
            attempts.map(|config_attempts| {

                match &envs_prefix {
                    Some(prefix) => {
                        retrying_variables = quote!(
                            #retrying_variables
                            let retrying_stop_attempts = ::retrying::overrite_by_env::<u32>(#config_attempts, #prefix, "STOP__ATTEMPTS");
                        );
                        stop_check = quote!((retrying_retry_attempt <= retrying_stop_attempts))
                    } 
                    None => stop_check = quote!((retrying_retry_attempt <= #config_attempts))
                }});
            

            duration.map(|config_duration| {

                if !stop_check.is_empty() {
                    stop_check=quote!(#stop_check &&);
                }

                match &envs_prefix {
                    Some(prefix) => {
                        retrying_variables = quote!(
                            #retrying_variables
                            let retrying_stop_duration = ::retrying::overrite_by_env::<u32>(#config_duration, #prefix, "STOP__DURATION");
                            let retrying_stop_duration_startime = ::std::time::SystemTime::now();
                        );
                        stop_check = quote!(#stop_check (::std::time::SystemTime::now().duration_since(retrying_stop_duration_startime).unwrap().as_secs() < (retrying_stop_duration as u64)));
                    },
                    None => {
                        retrying_variables = quote!(
                            #retrying_variables
                            let retrying_stop_duration_startime = ::std::time::SystemTime::now();
                        );
                        stop_check = quote!(#stop_check (::std::time::SystemTime::now().duration_since(retrying_stop_duration_startime).unwrap().as_secs() < (#config_duration as u64)));
                    }
                }
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

                    match &envs_prefix {
                        Some(prefix) => {
                            retrying_variables = quote!(
                                #retrying_variables
                                let retrying_wait_duration=::retrying::overrite_by_env::<u32>(#seconds, #prefix, "WAIT__FIXED");
                            )
                        },
                        None => 
                            retrying_variables = quote!(
                                #retrying_variables
                                let retrying_wait_duration=#seconds;
                            )
                    };

                    
                }
                WaitConfig::Random { min, max } => {
                    match &envs_prefix {
                        Some(prefix) => {
                            retrying_variables = quote!(
                                #retrying_variables
                                let retrying_wait_random_min=::retrying::overrite_by_env::<u32>(#min, #prefix, "WAIT__RANDOM__MIN");
                                let retrying_wait_random_max=::retrying::overrite_by_env::<u32>(#max, #prefix, "WAIT__RANDOM__MAX");
                                let mut retrying_wait_duration=0u32;
                            );
                            wait_duration_calc = quote!(
                                retrying_wait_duration = {       
                                    use ::retrying::rand::Rng;                 
                                    let mut retrying_wait_random_rng = ::retrying::rand::thread_rng();
                                    retrying_wait_random_rng.gen_range(retrying_wait_random_min..=retrying_wait_random_max) as u32
                                };
                            )

                        },
                        None => {
                            retrying_variables = quote!(
                                #retrying_variables
                                let mut retrying_wait_duration=0u32;   
                            );
                            wait_duration_calc = quote!(
                                retrying_wait_duration = {       
                                    use ::retrying::rand::Rng;                 
                                    let mut retrying_wait_random_rng = ::retrying::rand::thread_rng();
                                    retrying_wait_random_rng.gen_range(#min..=#max) as u32
                                };
                            )
                        }
                    }                  
                }
                WaitConfig::Exponential {
                    multiplier,
                    min,
                    max,
                    exp_base,
                } => {
                    match &envs_prefix {
                        Some(prefix) => {
                            retrying_variables = quote!(
                                #retrying_variables
                                let retrying_wait_exponential_multiplier=::retrying::overrite_by_env::<u32>(#multiplier, #prefix, "WAIT__EXPONENTIAL__MULTIPLIER");
                                let retrying_wait_exponential_min=::retrying::overrite_by_env::<u32>(#min, #prefix, "WAIT__EXPONENTIAL__MIN");
                                let retrying_wait_exponential_max=::retrying::overrite_by_env::<u32>(#max, #prefix, "WAIT__EXPONENTIAL__MAX");
                                let retrying_wait_exponential_exp_base=::retrying::overrite_by_env::<u32>(#exp_base, #prefix, "WAIT__EXPONENTIAL__EXP_BASE");
                                let mut retrying_wait_duration=0u32;
                            );
                            wait_duration_calc = quote!(
                                retrying_wait_duration = ::std::cmp::min(retrying_wait_exponential_multiplier * retrying_wait_exponential_exp_base.pow(retrying_retry_attempt - 1) + retrying_wait_exponential_min , retrying_wait_exponential_max);
                            )
                        },
                        None => {
                            retrying_variables = quote!(
                                #retrying_variables
                                let mut retrying_wait_duration=0u32;
                            );
        
                            wait_duration_calc = quote!(
                                retrying_wait_duration = ::std::cmp::min(#multiplier * #exp_base.pow(retrying_retry_attempt - 1) + #min , #max);
                            )
                        }
                    };

                    
                }
            };

            if asyncness.is_some() {
                wait_code = quote!(#wait_duration_calc
                    println!("Async wait {} seconds", retrying_wait_duration);
                    ::retrying::sleep_async(::retrying::Duration::from_secs(retrying_wait_duration as u64)).await;
                );
            } else {
                wait_code = quote!(#wait_duration_calc
                    println!("Sync wait {} seconds", retrying_wait_duration);
                    ::retrying::sleep_sync(::retrying::Duration::from_secs(retrying_wait_duration as u64));
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
