use crate::config::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemFn, Signature};

pub(crate) fn add_retry_code_into_function(
    function: ItemFn,
    config: RetryingConfig,
) -> TokenStream {
    let ItemFn {
        attrs,
        vis,
        sig,
        block,
        ..
    } = function;

    let Signature {
        constness,
        asyncness,
        unsafety,
        abi,
        fn_token,
        ident,
        generics:
            syn::Generics {
                params: gen_params,
                where_clause,
                ..
            },
        paren_token: _,
        inputs: params,
        variadic,
        output: return_type,
        ..
    } = sig;

    let RetryingConfig {
        stop,
        wait,
        retry,
        envs_prefix,
        ..
    } = config;

    let let_retrying_stop = stop.map_or(quote!(), |s| {
        let stop = prepare_stop(s, envs_prefix.clone());
        quote!(
            use ::retrying::stop::Stop;
            let retrying_stop = #stop;
        )
    });

    let retrying_stop_check = if let_retrying_stop.is_empty() {
        quote!(true)
    } else {
        quote!(!retrying_stop.stop_execution(&retrying_context))
    };

    let let_retrying_wait = wait.map_or(quote!(), |w| {
        let wait = prepare_wait(w, envs_prefix.clone());
        quote!(
            use ::retrying::wait::Wait;
            let retrying_wait = #wait;
        )
    });

    let retrying_wait = if !let_retrying_wait.is_empty() && asyncness.is_some() {
        quote!(::retrying::sleep_async(retrying_wait.wait_duration(&retrying_context)).await;)
    } else if !let_retrying_wait.is_empty() && asyncness.is_none() {
        quote!(::retrying::sleep_sync(retrying_wait.wait_duration(&retrying_context));)
    } else {
        quote!()
    };

    let retry_err_check = retry.map_or(quote!(), prepare_retry);

    quote!(
    #(#attrs) *
    #vis #constness #unsafety #asyncness #abi #fn_token #ident<#gen_params>(#params #variadic) #return_type
    #where_clause
    {
        let mut retrying_context = ::retrying::RetryingContext::new();
        #let_retrying_stop
        #let_retrying_wait

        loop {
            match #block {
                Ok(result) => return Ok(result),
                Err(err) if #retrying_stop_check => {
                    #retry_err_check
                    retrying_context.add_attempt();
                    #retrying_wait
                },
                Err(err) => break Err(err)
            }
        }
    })
}

fn prepare_stop(config: StopConfig, envs_prefix: Option<String>) -> TokenStream {
    let StopConfig { attempts, duration } = config;

    match (envs_prefix, attempts, duration) {
        (Some(prefix), Some(attempts), None) => {
            quote!(::retrying::stop::StopAttempts::new(::retrying::override_by_env::<u32>(#attempts, #prefix, ::retrying::envs::RETRYING_STOP_ATTEMPTS)))
        }
        (Some(prefix), None, Some(duration)) => {
            quote!(::retrying::stop::StopDuration::new(::retrying::override_by_env::<f32>(#duration, #prefix, ::retrying::envs::RETRYING_STOP_DURATION)))
        }
        (Some(prefix), Some(attempts), Some(duration)) => {
            quote!(::retrying::stop::StopAttemptsOrDuration::new(
                    ::retrying::override_by_env::<u32>(#attempts, #prefix, ::retrying::envs::RETRYING_STOP_ATTEMPTS),
                    ::retrying::override_by_env::<f32>(#duration, #prefix, ::retrying::envs::RETRYING_STOP_DURATION)
                )
            )
        }
        (None, Some(attempts), None) => {
            quote!(::retrying::stop::StopAttempts::new(#attempts))
        }
        (None, None, Some(duration)) => {
            quote!(::retrying::stop::StopDuration::new(#duration))
        }
        (None, Some(attempts), Some(duration)) => {
            quote!(::retrying::stop::StopAttemptsOrDuration::new(#attempts, #duration))
        }
        _ => quote!(::retrying::stop::StopNever {}),
    }
}

fn prepare_wait(config: WaitConfig, envs_prefix: Option<String>) -> TokenStream {
    match (config, envs_prefix) {
        (WaitConfig::Fixed { seconds }, Some(prefix)) => {
            quote!(::retrying::wait::WaitFixed::new(::retrying::override_by_env::<f32>(#seconds, #prefix, ::retrying::envs::RETRYING_WAIT_FIXED)))
        }
        (WaitConfig::Fixed { seconds }, None) => quote!(::retrying::wait::WaitFixed::new(#seconds)),

        (WaitConfig::Random { min, max }, Some(prefix)) => {
            quote!(::retrying::wait::WaitRandom::new(
                ::retrying::override_by_env::<f32>(#min, #prefix, ::retrying::envs::RETRYING_WAIT_RANDOM_MIN),
                ::retrying::override_by_env::<f32>(#max, #prefix, ::retrying::envs::RETRYING_WAIT_RANDOM_MAX)
            ))
        }
        (WaitConfig::Random { min, max }, None) => {
            quote!(::retrying::wait::WaitRandom::new(#min, #max))
        }
        (
            WaitConfig::Exponential {
                multiplier,
                min,
                max,
                exp_base,
            },
            Some(prefix),
        ) => quote!(::retrying::wait::WaitExponential::new(
            ::retrying::override_by_env::<f32>(#multiplier, #prefix, ::retrying::envs::RETRYING_WAIT_EXPONENTIAL_MULTIPLIER),
            ::retrying::override_by_env::<f32>(#min, #prefix, ::retrying::envs::RETRYING_WAIT_EXPONENTIAL_MIN),
            ::retrying::override_by_env::<f32>(#max, #prefix, ::retrying::envs::RETRYING_WAIT_EXPONENTIAL_MAX),
            ::retrying::override_by_env::<u32>(#exp_base, #prefix, ::retrying::envs::RETRYING_WAIT_EXPONENTIAL_EXP_BASE)
        )),
        (
            WaitConfig::Exponential {
                multiplier,
                min,
                max,
                exp_base,
            },
            None,
        ) => quote!(::retrying::wait::WaitExponential::new(#multiplier, #min, #max, #exp_base)),
    }
}

fn prepare_retry(config: RetryConfig) -> TokenStream {
    let RetryConfig {
        if_errors,
        if_not_errors,
    } = config;

    let if_error_check = if_errors.is_some();

    if let Some(errors) = if_errors.or(if_not_errors) {
        let errors_check = errors
            .iter()
            .map(|t| {
                let tkn: TokenStream = syn::parse_str(t.as_str()).unwrap();
                quote!(#tkn {..})
            })
            .reduce(|acc: TokenStream, v: TokenStream| quote!(#acc | #v));

        if if_error_check {
            quote!(
                match err {
                    #errors_check => (),
                    _ => break Err(err)
                };
            )
        } else {
            quote!(
                match err {
                    #errors_check => break Err(err),
                    _ => ()
                };
            )
        }
    } else {
        quote!()
    }
}

#[cfg(test)]
mod tests {
    use crate::code_gen::*;

    #[test]
    fn test_prepare_stop() {
        let result = prepare_stop(
            StopConfig {
                attempts: Some(1),
                duration: None,
            },
            None,
        );
        assert_eq!(
            result.to_string(),
            ":: retrying :: stop :: StopAttempts :: new (1u32)"
        );

        let result = prepare_stop(
            StopConfig {
                attempts: Some(1),
                duration: None,
            },
            Some("TEST".to_string()),
        );
        assert_eq!(result.to_string(), ":: retrying :: stop :: StopAttempts :: new (:: retrying :: override_by_env :: < u32 > (1u32 , \"TEST\" , :: retrying :: envs :: RETRYING_STOP_ATTEMPTS))");

        let result = prepare_stop(
            StopConfig {
                attempts: None,
                duration: Some(1.5),
            },
            None,
        );
        assert_eq!(
            result.to_string(),
            ":: retrying :: stop :: StopDuration :: new (1.5f32)"
        );

        let result = prepare_stop(
            StopConfig {
                attempts: None,
                duration: Some(1.5),
            },
            Some("TEST".to_string()),
        );
        assert_eq!(result.to_string(), ":: retrying :: stop :: StopDuration :: new (:: retrying :: override_by_env :: < f32 > (1.5f32 , \"TEST\" , :: retrying :: envs :: RETRYING_STOP_DURATION))");

        let result = prepare_stop(
            StopConfig {
                attempts: Some(1),
                duration: Some(0.5),
            },
            None,
        );
        assert_eq!(
            result.to_string(),
            ":: retrying :: stop :: StopAttemptsOrDuration :: new (1u32 , 0.5f32)"
        );

        let result = prepare_stop(
            StopConfig {
                attempts: Some(1),
                duration: Some(0.5),
            },
            Some("TEST".to_string()),
        );
        assert_eq!(result.to_string(), ":: retrying :: stop :: StopAttemptsOrDuration :: new (\
            :: retrying :: override_by_env :: < u32 > (1u32 , \"TEST\" , :: retrying :: envs :: RETRYING_STOP_ATTEMPTS) , \
            :: retrying :: override_by_env :: < f32 > (0.5f32 , \"TEST\" , :: retrying :: envs :: RETRYING_STOP_DURATION))");
    }

    #[test]
    fn test_prepare_wait() {
        let result = prepare_wait(WaitConfig::Fixed { seconds: 0.5 }, None);
        assert_eq!(
            result.to_string(),
            ":: retrying :: wait :: WaitFixed :: new (0.5f32)"
        );

        let result = prepare_wait(WaitConfig::Fixed { seconds: 0.5 }, Some("TEST".to_string()));
        assert_eq!(result.to_string(), ":: retrying :: wait :: WaitFixed :: new (:: retrying :: override_by_env :: < f32 > (0.5f32 , \"TEST\" , :: retrying :: envs :: RETRYING_WAIT_FIXED))");

        let result = prepare_wait(
            WaitConfig::Random {
                min: 0.1,
                max: 100.0,
            },
            None,
        );
        assert_eq!(
            result.to_string(),
            ":: retrying :: wait :: WaitRandom :: new (0.1f32 , 100f32)"
        );

        let result = prepare_wait(
            WaitConfig::Random {
                min: 0.1,
                max: 100.0,
            },
            Some("TEST".to_string()),
        );
        assert_eq!(result.to_string(), ":: retrying :: wait :: WaitRandom :: new (\
            :: retrying :: override_by_env :: < f32 > (0.1f32 , \"TEST\" , :: retrying :: envs :: RETRYING_WAIT_RANDOM_MIN) , \
            :: retrying :: override_by_env :: < f32 > (100f32 , \"TEST\" , :: retrying :: envs :: RETRYING_WAIT_RANDOM_MAX))");

        let result = prepare_wait(
            WaitConfig::Exponential {
                multiplier: 0.5,
                min: 0.5,
                max: 1.5,
                exp_base: 2,
            },
            None,
        );
        assert_eq!(
            result.to_string(),
            ":: retrying :: wait :: WaitExponential :: new (0.5f32 , 0.5f32 , 1.5f32 , 2u32)"
        );

        let result = prepare_wait(
            WaitConfig::Exponential {
                multiplier: 0.5,
                min: 0.5,
                max: 1.5,
                exp_base: 2,
            },
            Some("TEST".to_string()),
        );
        assert_eq!(result.to_string(), ":: retrying :: wait :: WaitExponential :: new (\
            :: retrying :: override_by_env :: < f32 > (0.5f32 , \"TEST\" , :: retrying :: envs :: RETRYING_WAIT_EXPONENTIAL_MULTIPLIER) , \
            :: retrying :: override_by_env :: < f32 > (0.5f32 , \"TEST\" , :: retrying :: envs :: RETRYING_WAIT_EXPONENTIAL_MIN) , \
            :: retrying :: override_by_env :: < f32 > (1.5f32 , \"TEST\" , :: retrying :: envs :: RETRYING_WAIT_EXPONENTIAL_MAX) , \
            :: retrying :: override_by_env :: < u32 > (2u32 , \"TEST\" , :: retrying :: envs :: RETRYING_WAIT_EXPONENTIAL_EXP_BASE))");
    }

    #[test]
    fn test_prepare_retry() {
        let result = prepare_retry(RetryConfig {
            if_errors: Some(vec!["syn::Error".to_string()]),
            if_not_errors: None,
        });
        assert_eq!(
            result.to_string(),
            "match err { syn :: Error { .. } => () , _ => break Err (err) } ;"
        );

        let result = prepare_retry(RetryConfig {
            if_errors: None,
            if_not_errors: Some(vec!["syn::Error".to_string(), "::other::Error".to_string()]),
        });
        assert_eq!(result.to_string(), "match err { syn :: Error { .. } | :: other :: Error { .. } => break Err (err) , _ => () } ;");
    }

    #[test]
    fn test_add_retry_code_into_function_all_config() {
        let config = RetryingConfig {
            stop: Some(StopConfig {
                attempts: Some(1),
                duration: Some(5.5),
            }),
            wait: Some(WaitConfig::Fixed { seconds: 0.5 }),
            retry: Some(RetryConfig {
                if_errors: Some(vec![
                    ":: syn :: Error".to_string(),
                    ":: std :: num :: ParseIntError".to_string(),
                ]),
                if_not_errors: None,
            }),
            envs_prefix: Some(String::from("TEST")),
        };

        let function = syn::parse_quote!(
            fn test_function(in_param: &str) -> Result<i32, ParseIntError> {
                in_param.parse::<i32>()
            }
        );

        let result = add_retry_code_into_function(function, config);
        let expected = "\
        fn test_function < > (in_param : & str) -> Result < i32 , ParseIntError > { \
            let mut retrying_context = :: retrying :: RetryingContext :: new () ; \
            use :: retrying :: stop :: Stop ; \
            let retrying_stop = :: retrying :: stop :: StopAttemptsOrDuration :: new (\
                :: retrying :: override_by_env :: < u32 > (1u32 , \"TEST\" , :: retrying :: envs :: RETRYING_STOP_ATTEMPTS) , \
                :: retrying :: override_by_env :: < f32 > (5.5f32 , \"TEST\" , :: retrying :: envs :: RETRYING_STOP_DURATION)\
            ) ; \
            use :: retrying :: wait :: Wait ; \
            let retrying_wait = :: retrying :: wait :: WaitFixed :: new (:: retrying :: override_by_env :: < f32 > (0.5f32 , \"TEST\" , :: retrying :: envs :: RETRYING_WAIT_FIXED)) ; \
            loop { match { in_param . parse :: < i32 > () } { \
                Ok (result) => return Ok (result) , \
                Err (err) if ! retrying_stop . stop_execution (& retrying_context) => { \
                    match err { \
                        :: syn :: Error { .. } | :: std :: num :: ParseIntError { .. } => () , \
                        _ => break Err (err) \
                    } ; \
                    retrying_context . add_attempt () ; \
                    :: retrying :: sleep_sync (retrying_wait . wait_duration (& retrying_context)) ; \
                } , \
                Err (err) => break Err (err) \
            } \
        } }";
        assert_eq!(result.to_string(), expected);
    }

    #[test]
    fn test_add_retry_code_into_function_no_config() {
        let config = RetryingConfig {
            stop: None,
            wait: None,
            retry: None,
            envs_prefix: None,
        };

        let function = syn::parse_quote!(
            fn test_function(in_param: &str) -> Result<i32, ParseIntError> {
                in_param.parse::<i32>()
            }
        );

        let result = add_retry_code_into_function(function, config);

        let expected = "\
        fn test_function < > (in_param : & str) -> Result < i32 , ParseIntError > { \
            let mut retrying_context = :: retrying :: RetryingContext :: new () ; \
            loop { match { in_param . parse :: < i32 > () } { \
                Ok (result) => return Ok (result) , \
                Err (err) if true => { \
                    retrying_context . add_attempt () ; \
                } , \
                Err (err) => break Err (err) \
            } \
        } }";

        assert_eq!(result.to_string(), expected);
    }
}
