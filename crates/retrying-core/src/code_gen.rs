use crate::config::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemFn, Signature};

pub(crate) fn add_retry_code_into_function(
    function: ItemFn,
    config: RetryingConfig,
) -> TokenStream {
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

    let RetryingConfig {
        stop,
        wait,
        retry,
        envs_prefix,
        ..
    } = config;

    let mut retrying_variables = quote!(let mut retrying_retry_attempt=1u32;);
    let retrying_end_of_loop_cycle = quote!(retrying_retry_attempt+=1u32;);

    let mut stop_check = quote!(true);

    if let Some(StopConfig { attempts, duration }) = stop {
        if let Some(config_attempts) = attempts {
            match &envs_prefix {
                Some(prefix) => {
                    retrying_variables = quote!(
                        #retrying_variables
                        let retrying_stop_attempts = ::retrying::override_by_env::<u32>(#config_attempts, #prefix, ::retrying::envs::RETRYING_STOP_ATTEMPTS);
                    );
                    stop_check = quote!((retrying_retry_attempt <= retrying_stop_attempts))
                }
                None => stop_check = quote!((retrying_retry_attempt <= #config_attempts)),
            }
        };

        if let Some(config_duration) = duration {
            if !stop_check.is_empty() {
                stop_check = quote!(#stop_check &&);
            }

            match &envs_prefix {
                Some(prefix) => {
                    retrying_variables = quote!(
                        #retrying_variables
                        let retrying_stop_duration = ::retrying::override_by_env::<f32>(#config_duration, #prefix, ::retrying::envs::RETRYING_STOP_DURATION);
                        let retrying_stop_duration_startime = ::std::time::SystemTime::now();
                    );
                    stop_check = quote!(#stop_check (::std::time::SystemTime::now().duration_since(retrying_stop_duration_startime).unwrap().as_secs_f32() < retrying_stop_duration));
                }
                None => {
                    retrying_variables = quote!(
                        #retrying_variables
                        let retrying_stop_duration_startime = ::std::time::SystemTime::now();
                    );
                    stop_check = quote!(#stop_check (::std::time::SystemTime::now().duration_since(retrying_stop_duration_startime).unwrap().as_secs_f32() < #config_duration));
                }
            }
        };
    };

    let mut wait_code = quote!();

    if let Some(wait_config) = wait {
        let mut wait_duration_calc = quote!();

        match wait_config {
            WaitConfig::Fixed { seconds } => {
                match &envs_prefix {
                    Some(prefix) => {
                        retrying_variables = quote!(
                            #retrying_variables
                            let retrying_wait_duration=::retrying::override_by_env::<f32>(#seconds, #prefix, ::retrying::envs::RETRYING_WAIT_FIXED);
                        )
                    }
                    None => {
                        retrying_variables = quote!(
                            #retrying_variables
                            let retrying_wait_duration=#seconds;
                        )
                    }
                };
            }
            WaitConfig::Random { min, max } => match &envs_prefix {
                Some(prefix) => {
                    retrying_variables = quote!(
                        #retrying_variables
                        let retrying_wait_random_min=::retrying::override_by_env::<f32>(#min, #prefix, ::retrying::envs::RETRYING_WAIT_RANDOM_MIN);
                        let retrying_wait_random_max=::retrying::override_by_env::<f32>(#max, #prefix, ::retrying::envs::RETRYING_WAIT_RANDOM_MAX);
                        let mut retrying_wait_duration=0f32;
                    );
                    wait_duration_calc = quote!(
                        retrying_wait_duration = {
                            use ::retrying::rand::Rng;
                            let mut retrying_wait_random_rng = ::retrying::rand::thread_rng();
                            retrying_wait_random_rng.gen_range(retrying_wait_random_min..=retrying_wait_random_max) as f32
                        };
                    )
                }
                None => {
                    retrying_variables = quote!(
                        #retrying_variables
                        let mut retrying_wait_duration=0f32;
                    );
                    wait_duration_calc = quote!(
                        retrying_wait_duration = {
                            use ::retrying::rand::Rng;
                            let mut retrying_wait_random_rng = ::retrying::rand::thread_rng();
                            retrying_wait_random_rng.gen_range(#min..=#max) as f32
                        };
                    )
                }
            },
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
                            let retrying_wait_exponential_multiplier=::retrying::override_by_env::<f32>(#multiplier, #prefix, ::retrying::envs::RETRYING_WAIT_EXPONENTIAL_MULTIPLIER);
                            let retrying_wait_exponential_min=::retrying::override_by_env::<f32>(#min, #prefix, ::retrying::envs::RETRYING_WAIT_EXPONENTIAL_MIN);
                            let retrying_wait_exponential_max=::retrying::override_by_env::<f32>(#max, #prefix, ::retrying::envs::RETRYING_WAIT_EXPONENTIAL_MAX);
                            let retrying_wait_exponential_exp_base=::retrying::override_by_env::<u32>(#exp_base, #prefix, ::retrying::envs::RETRYING_WAIT_EXPONENTIAL_EXP_BASE);
                            let mut retrying_wait_duration=0f32;
                        );
                        wait_duration_calc = quote!(
                            retrying_wait_duration = retrying_wait_exponential_max.min(retrying_wait_exponential_multiplier * (retrying_wait_exponential_exp_base.powf(retrying_retry_attempt - 1) as f32) + retrying_wait_exponential_min);
                        )
                    }
                    None => {
                        retrying_variables = quote!(
                            #retrying_variables
                            let mut retrying_wait_duration=0f32;
                        );

                        wait_duration_calc = quote!(
                            retrying_wait_duration = #max.min(#multiplier * (#exp_base.pow(retrying_retry_attempt - 1) as f32) + #min);
                        )
                    }
                };
            }
        };

        if asyncness.is_some() {
            wait_code = quote!(#wait_duration_calc
                println!("Async wait {} seconds", retrying_wait_duration);
                ::retrying::sleep_async(::retrying::Duration::from_secs_f32(retrying_wait_duration)).await;
            );
        } else {
            wait_code = quote!(#wait_duration_calc
                println!("Sync wait {} seconds", retrying_wait_duration);
                ::retrying::sleep_sync(::retrying::Duration::from_secs_f32(retrying_wait_duration));
            );
        }
    }

    let mut retry_err_check = quote!();

    if let Some(RetryConfig {
        if_errors,
        if_not_errors,
    }) = retry
    {
        if let Some(configured_errors) = if_errors {
            let mut errors_check = quote!();
            for err in configured_errors {
                if errors_check.is_empty() {
                    errors_check = quote!(#err {..});
                } else {
                    errors_check = quote!(#errors_check | #err {..});
                }
            }

            retry_err_check = quote!(
                match err {
                    #errors_check => (),
                    _ => break Err(err)
                };
            )
        } else if let Some(configured_errors) = if_not_errors {
            let mut errors_check = quote!();
            for err in configured_errors {
                if errors_check.is_empty() {
                    errors_check = quote!(#err {..});
                } else {
                    errors_check = quote!(#errors_check | #err {..});
                }
            }

            retry_err_check = quote!(
                match err {
                    #errors_check => break Err(err),
                    _ => ()
                };
            )
        }
    };

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
                    #retry_err_check
                    println!("New attempt");
                    #wait_code
                },
                Err(err) => break Err(err)
            }
            #retrying_end_of_loop_cycle
        }
    })
}
