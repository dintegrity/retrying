use crate::errors::RetryConfigurationError;
use proc_macro2::TokenStream;
use quote::quote;
use std::fmt::{self, Debug};
use std::str::FromStr;
use syn::parse::Parser;
use syn::punctuated::Punctuated;

// In syn 2.0 AttributeArgs was removed, so now we can use type alias to simplify syntaxis
type AttributeArgs = syn::punctuated::Punctuated<syn::Meta, syn::Token![,]>;

#[derive(Debug, PartialEq)]
pub enum WaitConfig {
    Fixed {
        seconds: f32,
    },
    Random {
        min: f32,
        max: f32,
    },
    Exponential {
        multiplier: f32,
        min: f32,
        max: f32,
        exp_base: u32,
    },
}

impl WaitConfig {
    const FIXED: &'static str = "fixed";
    const RANDOM: &'static str = "random";
    const EXPONENTIAL: &'static str = "exponential";
    const MIN: &'static str = "min";
    const MAX: &'static str = "max";
    const EXP_BASE: &'static str = "exp_base";
    const MULTIPLIER: &'static str = "multiplier";
}

#[derive(Debug, PartialEq)]
pub(crate) struct StopConfig {
    pub(crate) attempts: Option<u32>,
    pub(crate) duration: Option<f32>,
}
impl StopConfig {
    const ATTEMPTS: &'static str = "attempts";
    const DURATION: &'static str = "duration";
}

#[derive(Debug, PartialEq)]
pub(crate) struct RetryConfig {
    pub(crate) if_errors: Option<Vec<String>>,
    pub(crate) if_not_errors: Option<Vec<String>>,
}

impl RetryConfig {
    const IF_ERRORS: &'static str = "if_errors";
    const IF_NOT_ERRORS: &'static str = "if_not_errors";
}

#[derive(Debug, PartialEq)]
pub(crate) struct RetryingConfig {
    pub(crate) stop: Option<StopConfig>,
    pub(crate) wait: Option<WaitConfig>,
    pub(crate) retry: Option<RetryConfig>,
    pub(crate) envs_prefix: Option<String>,
}

impl RetryingConfig {
    const STOP: &'static str = "stop";
    const WAIT: &'static str = "wait";
    const RETRY: &'static str = "retry";
    const ENVS_PREFIX: &'static str = "envs_prefix";

    fn new() -> RetryingConfig {
        RetryingConfig {
            stop: None,
            wait: None,
            retry: None,
            envs_prefix: None,
        }
    }

    fn stop(&mut self, expr: syn::Expr) -> Result<(), RetryConfigurationError> {
        let parsed_config = Self::parse_stop_config(expr)?;

        if parsed_config.attempts.is_some() || parsed_config.duration.is_some() {
            self.stop = Some(parsed_config);
        }
        Ok(())
    }

    fn parse_stop_config(expr: syn::Expr) -> Result<StopConfig, RetryConfigurationError> {
        let mut attempts = None;
        let mut duration = None;
        let functions = parse_functions_expr(expr)?;

        for func in functions {
            match func.ident.as_str() {
                StopConfig::ATTEMPTS => attempts = func.args.first().map(|arg|arg.value.to_string().parse::<u32>().unwrap()),
                StopConfig::DURATION => duration = func.args.first().map(|arg|arg.value.to_string().parse::<f32>().unwrap()),
                unknown => return Err(RetryConfigurationError::new(
                    format!("Configuration {} is wrong for `{}`. Possible configuration option is `{}` and `{}`", unknown, RetryingConfig::STOP, StopConfig::ATTEMPTS, StopConfig::DURATION)))
            }
        }
        Ok(StopConfig { attempts, duration })
    }

    fn wait(&mut self, expr: syn::Expr) -> Result<(), RetryConfigurationError> {
        let parsed_config = Self::parse_wait_config(expr)?;
        self.wait = Some(parsed_config);
        Ok(())
    }

    fn parse_wait_config(expr: syn::Expr) -> Result<WaitConfig, RetryConfigurationError> {
        let ParsedFunction { ident, args } = parse_function_call(expr)?;
        match ident.as_str() {
            WaitConfig::FIXED => {
                if args.len() > 1 || args.first().filter(|x| x.ident.is_some()).is_some() {
                    Err(RetryConfigurationError::new(format!(
                        "{}={} has only one argument without name.",
                        RetryingConfig::WAIT,
                        WaitConfig::FIXED
                    )))
                } else {
                    let seconds = args
                        .first()
                        .map(|x| x.value.parse::<f32>().unwrap())
                        .unwrap_or(0f32);
                    Ok(WaitConfig::Fixed { seconds })
                }
            }
            WaitConfig::RANDOM => {
                let mut min: f32 = 0.0;
                let mut max: f32 = 3600.0;

                for FunctionArgument { ident, value } in args {
                    match ident.unwrap_or(String::new()).as_str() {
                        WaitConfig::MIN => min = value.parse::<f32>()?,
                        WaitConfig::MAX => max = value.parse::<f32>()?,
                        unknown => return Err(RetryConfigurationError::new(format!("{}={} has wrong configuration {}. Only `{}` and `{}` attributes is possible", RetryingConfig::WAIT, WaitConfig::RANDOM, unknown, WaitConfig::MIN, WaitConfig::MAX))),           
                    }
                }
                Ok(WaitConfig::Random { min, max })
            }
            WaitConfig::EXPONENTIAL => {
                let mut min: f32 = 0.0;
                let mut max: f32 = 3600.0;
                let mut multiplier: f32 = 1.0;
                let mut exp_base: u32 = 2;

                for FunctionArgument { ident, value } in args {
                    match ident.unwrap_or(String::new()).as_str() {
                        WaitConfig::MIN => min = value.parse::<f32>()?,
                        WaitConfig::MAX => max = value.parse::<f32>()?,
                        WaitConfig::MULTIPLIER => multiplier = value.parse::<f32>()?,
                        WaitConfig::EXP_BASE => exp_base = value.parse::<u32>()?,
                        unknown => return Err(RetryConfigurationError::new(format!("{}={} has wrong configuration option `{}`. Only `{}`, `{}`, `{}` and `{}` attributes is possible", RetryingConfig::WAIT, WaitConfig::EXPONENTIAL, unknown, WaitConfig::MIN, WaitConfig::MAX, WaitConfig::EXPONENTIAL, WaitConfig::EXP_BASE))),
                    }
                }

                Ok(WaitConfig::Exponential {
                    multiplier,
                    min,
                    max,
                    exp_base,
                })
            }
            unknown => Err(RetryConfigurationError::new(format!(
                "Configuration {} is wrong for `{}`. Possible configuration is `{}`, `{}` and `{}`",
                unknown,
                RetryingConfig::WAIT,
                WaitConfig::FIXED,
                WaitConfig::RANDOM,
                WaitConfig::EXPONENTIAL
            ))),
        }
    }

    fn retry(&mut self, expr: syn::Expr) -> Result<(), RetryConfigurationError> {
        let parsed_config = Self::parse_retry_config(expr)?;
        self.retry = Some(parsed_config);
        Ok(())
    }

    fn parse_retry_config(expr: syn::Expr) -> Result<RetryConfig, RetryConfigurationError> {
        let mut if_errors: Option<Vec<String>> = None;
        let mut if_not_errors: Option<Vec<String>> = None;

        let functions = parse_functions_expr(expr)?;

        for func in functions {
            let parsed_args = func.args.iter().map(|a| a.value.parse().unwrap()).collect();
            if !func.args.is_empty() {
                match func.ident.as_str() {
                    RetryConfig::IF_ERRORS => if_errors = Some(parsed_args),
                    RetryConfig::IF_NOT_ERRORS => if_not_errors = Some(parsed_args),
                    unknown => return Err(RetryConfigurationError::new(format!("Configuration {} is wrong for `{}`. Possible configuration option is `{}` and `{}`", unknown, RetryingConfig::RETRY, RetryConfig::IF_ERRORS, RetryConfig::IF_NOT_ERRORS)))
                }
            }
        }

        if if_errors.is_some() && if_not_errors.is_some() {
            Err(RetryConfigurationError::new(format!("Configuration is wrong for `{}`. Only one of `{}` and `{}` should be configured at the same time", RetryingConfig::RETRY, RetryConfig::IF_ERRORS, RetryConfig::IF_NOT_ERRORS)))
        } else {
            Ok(RetryConfig {
                if_errors,
                if_not_errors,
            })
        }
    }

    fn envs_prefix(&mut self, expr: syn::Expr) -> Result<(), RetryConfigurationError> {
        let parsed_config = Self::parse_envs_prefix_config(expr)?;

        self.envs_prefix = Some(parsed_config);
        Ok(())
    }

    fn parse_envs_prefix_config(expr: syn::Expr) -> Result<String, RetryConfigurationError> {
        match parse_value(expr) {
            Ok(ParsedValue::ParsedString(v)) => Ok(v),
            _ => Err(RetryConfigurationError::new(format!(
                "`{}` value should be string literal (for exampe `envs_prefix=\"retry\"`)",
                RetryingConfig::ENVS_PREFIX
            ))),
        }
    }

    pub(crate) fn from_token_stream(
        args: TokenStream,
    ) -> Result<RetryingConfig, RetryConfigurationError> {
        let mut config = RetryingConfig::new();

        let args = AttributeArgs::parse_terminated.parse2(args).or(Err(
            RetryConfigurationError::from_str("Can't parse comma delimeted retry configuration"),
        ))?;

        for arg in args {
            match arg {
                    syn::Meta::NameValue(name_value) => {
                        let ident = name_value
                            .path
                            .get_ident()
                            .ok_or_else(|| {
                                RetryConfigurationError::from_str("Named value without ident")
                            })
                            .map(|v| v.to_string().to_lowercase())?;
                        let value = name_value.value;

                        match ident.as_str()  {
                            RetryingConfig::STOP => config.stop(value)?,
                            RetryingConfig::WAIT => config.wait(value)?,
                            RetryingConfig::RETRY => config.retry(value)?,
                            RetryingConfig::ENVS_PREFIX => config.envs_prefix(value)?,
                            unknown => return Err(RetryConfigurationError::new(format!("Unknown configuration  option`{}`. Possible values `{}`,`{}`, `{}`, `{}`.", unknown, RetryingConfig::STOP, RetryingConfig::WAIT, RetryingConfig::RETRY, RetryingConfig::ENVS_PREFIX)))
                        }
                    }
                    _ => {
                        return Err(RetryConfigurationError::from_str(
                            "Unknown format of configuration options. Only `name=value` is acceptable in retry config.",
                        ))
                    }
                }
        }
        Ok(config)
    }
}

#[derive(Debug, PartialEq)]
enum ParsedValue {
    ParsedInt(u32),
    ParsedString(String),
    ParsedBool(bool),
    ParseFloat(f32),
    ParsedPath(String),
}

impl fmt::Display for ParsedValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParsedValue::ParsedInt(v) => write!(f, "{v}"),
            ParsedValue::ParsedString(v) => write!(f, "{v}"),
            ParsedValue::ParsedBool(v) => write!(f, "{v}"),
            ParsedValue::ParseFloat(v) => write!(f, "{v}"),
            ParsedValue::ParsedPath(v) => write!(f, "{v}"),
        }
    }
}

impl ParsedValue {
    fn parse<T: FromStr>(&self) -> Result<T, RetryConfigurationError> {
        self.to_string()
            .parse::<T>()
            .map_err(|_| RetryConfigurationError::from_str("Failed cast to literal"))
    }
}

#[derive(Debug, PartialEq)]
struct FunctionArgument {
    ident: Option<String>,
    value: ParsedValue,
}

#[derive(Debug, PartialEq)]
struct ParsedFunction {
    ident: String,
    args: Vec<FunctionArgument>,
}

fn parse_functions_expr(
    functions_expr: syn::Expr,
) -> Result<Vec<ParsedFunction>, RetryConfigurationError> {
    match functions_expr {
        syn::Expr::Paren(syn::ExprParen { expr, .. }) => {
            let deref_expr = *expr;
            match deref_expr {
                syn::Expr::Binary(syn::ExprBinary{left, right, op, ..}) => match op {
                    syn::BinOp::BitOr(_) => {
                        let left = *left;
                        let right = *right;
                        let parsed_left = parse_function_call(left)?;
                        let parsed_right = parse_function_call(right)?;
                        Ok(vec![parsed_left, parsed_right])
                    },
                    _ => Err(RetryConfigurationError::from_str("Incorrect symbol between configuration functions. Supported only bit or (`|`). For example, `function(args)|function2(args)"))
                },
                syn::Expr::Call(_) => {
                    let parsed_function = parse_function_call(deref_expr)?;
                    Ok(vec![parsed_function])
                }
                _ => Err(RetryConfigurationError::from_str("Incorrect expression between paren. Supported only one function `function(args)` or multiple functions `function(args)|function2(args)"))
            }
        }
        _ => {
            let parsed_function = parse_function_call(functions_expr)?;
            Ok(vec![parsed_function])
        }
    }
}

fn parse_function_call(
    function_expr: syn::Expr,
) -> Result<ParsedFunction, RetryConfigurationError> {
    match function_expr {
        syn::Expr::Call(syn::ExprCall { func, args, .. }) => {
            let ident = parse_ident(*func)?;
            let args: Vec<FunctionArgument> = parse_function_arguments(args)?;
            Ok(ParsedFunction { ident, args })
        }
        _ => Err(RetryConfigurationError::from_str("Not a function")),
    }
}

fn parse_function_arguments(
    arguments: Punctuated<syn::Expr, syn::Token![,]>,
) -> Result<Vec<FunctionArgument>, RetryConfigurationError> {
    let mut parsed_arguments: Vec<FunctionArgument> = Vec::new();

    for arg in arguments {
        match arg {
            syn::Expr::Assign(syn::ExprAssign { left, right, .. }) => {
                let name = parse_ident(*left)?;
                let parsed_value = parse_value(*right)?;
                parsed_arguments.push(FunctionArgument {
                    ident: Some(name),
                    value: parsed_value,
                });
            }
            expr => {
                let parsed_value = parse_value(expr)?;
                parsed_arguments.push(FunctionArgument {
                    ident: None,
                    value: parsed_value,
                });
            }
        }
    }
    Ok(parsed_arguments)
}

fn parse_value(expr: syn::Expr) -> Result<ParsedValue, RetryConfigurationError> {
    match expr {
        syn::Expr::Path(syn::ExprPath { path, .. }) => {
            Ok(ParsedValue::ParsedPath(quote!(#path).to_string()))
        }
        syn::Expr::Lit(syn::ExprLit { lit, .. }) => match lit {
            syn::Lit::Int(lit) => match lit.base10_parse::<u32>() {
                Ok(value) => Ok(ParsedValue::ParsedInt(value)),
                Err(e) => Err(RetryConfigurationError::new(format!(
                    "Failed to parse LitInt to `u32`. Error: {}",
                    e
                ))),
            },
            syn::Lit::Str(s) => Ok(ParsedValue::ParsedString(s.value())),
            syn::Lit::Verbatim(s) => Ok(ParsedValue::ParsedString(s.to_string())),
            syn::Lit::Bool(b) => Ok(ParsedValue::ParsedBool(b.value)),
            syn::Lit::Float(b) => match b.base10_parse::<f32>() {
                Ok(value) => Ok(ParsedValue::ParseFloat(value)),
                Err(e) => Err(RetryConfigurationError::new(format!(
                    "Failed to parse LitFloat to f32. Error: {}",
                    e
                ))),
            },
            _ => Err(RetryConfigurationError::from_str(
                "Unsupported literal. Currently supported only Int, Str, Verbatim, Bool and Float",
            )),
        },
        _ => Err(RetryConfigurationError::from_str(
            "Unsupported value. Currently supported only Path, Int, Str, Verbatim, Bool and Float",
        )),
    }
}

fn parse_ident(expr: syn::Expr) -> Result<String, RetryConfigurationError> {
    match expr {
        syn::Expr::Path(syn::ExprPath { path, .. }) => path
            .get_ident()
            .ok_or_else(|| RetryConfigurationError::from_str("Named value without ident"))
            .map(|v| v.to_string()),
        _ => Err(RetryConfigurationError::from_str(
            "Incorrect expression in parse_ident",
        )),
    }
}

#[cfg(test)]
mod tests {
    use crate::config::*;
    use quote::quote;
    use std::vec;

    #[test]
    fn test_parse_stop_config() {
        let mut config = RetryingConfig::new();
        config.stop(syn::parse_quote!(attempts(5))).unwrap();

        assert_eq!(
            config.stop,
            Some(StopConfig {
                attempts: Some(5),
                duration: None
            })
        );

        config.stop(syn::parse_quote!(duration(0.5))).unwrap();
        assert_eq!(
            config.stop,
            Some(StopConfig {
                attempts: None,
                duration: Some(0.5)
            })
        );

        config
            .stop(syn::parse_quote!((attempts(5) | duration(0.5))))
            .unwrap();
        assert_eq!(
            config.stop,
            Some(StopConfig {
                attempts: Some(5),
                duration: Some(0.5)
            })
        );
    }

    #[test]
    fn test_parse_wait_config() {
        let mut config = RetryingConfig::new();

        config.wait(syn::parse_quote!(fixed(4.4))).unwrap();
        assert_eq!(config.wait, Some(WaitConfig::Fixed { seconds: 4.4 }));

        config
            .wait(syn::parse_quote!(random(min = 0.4, max = 1.5)))
            .unwrap();
        assert_eq!(config.wait, Some(WaitConfig::Random { min: 0.4, max: 1.5 }));

        config
            .wait(syn::parse_quote!(exponential(
                min = 0.4,
                max = 1.5,
                multiplier = 1.2,
                exp_base = 2
            )))
            .unwrap();
        assert_eq!(
            config.wait,
            Some(WaitConfig::Exponential {
                multiplier: 1.2,
                min: 0.4,
                max: 1.5,
                exp_base: 2
            })
        );
    }

    #[test]
    fn test_parse_retry_config() {
        let mut config = RetryingConfig::new();

        config
            .retry(syn::parse_quote!(if_errors(
                syn::Err,
                ::std::num::ParseIntError
            )))
            .unwrap();
        assert_eq!(
            config.retry,
            Some(RetryConfig {
                if_errors: Some(vec![
                    "syn :: Err".to_string(),
                    ":: std :: num :: ParseIntError".to_string()
                ]),
                if_not_errors: None
            })
        );

        config
            .retry(syn::parse_quote!(if_not_errors(
                syn::Err,
                ::std::num::ParseIntError
            )))
            .unwrap();
        assert_eq!(
            config.retry,
            Some(RetryConfig {
                if_errors: None,
                if_not_errors: Some(vec![
                    "syn :: Err".to_string(),
                    ":: std :: num :: ParseIntError".to_string()
                ])
            })
        );
    }

    #[test]
    fn test_parse_envs_prefix_config() {
        let mut config = RetryingConfig::new();

        config.envs_prefix(syn::parse_quote!("TEST")).unwrap();
        assert_eq!(config.envs_prefix, Some("TEST".to_string()));
    }

    #[test]
    fn test_from_token_stream() {
        let token_stream = quote!(
            stop = (attempts(1) | duration(5.5)),
            wait = fixed(0.5),
            retry = if_errors(::syn::Error, ::std::num::ParseIntError),
            envs_prefix = "TEST"
        );

        let expected = RetryingConfig {
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

        let result = RetryingConfig::from_token_stream(token_stream).unwrap();

        assert_eq!(result, expected);

        let result = RetryingConfig::from_token_stream(quote!()).unwrap();
        assert_eq!(result, RetryingConfig::new());
    }

    #[test]
    fn test_parse_functions_expr() {
        let expected = vec![
            ParsedFunction {
                ident: String::from("function1"),
                args: vec![FunctionArgument {
                    ident: None,
                    value: ParsedValue::ParsedInt(1),
                }],
            },
            ParsedFunction {
                ident: String::from("function2"),
                args: vec![FunctionArgument {
                    ident: Some(String::from("test")),
                    value: ParsedValue::ParseFloat(5.5),
                }],
            },
        ];
        let result =
            parse_functions_expr(syn::parse_quote!((function1(1) | function2(test = 5.5))))
                .unwrap();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_parse_function_call() {
        let expected = ParsedFunction {
            ident: String::from("function1"),
            args: vec![
                FunctionArgument {
                    ident: None,
                    value: ParsedValue::ParsedInt(1),
                },
                FunctionArgument {
                    ident: Some(String::from("test")),
                    value: ParsedValue::ParseFloat(2.4),
                },
            ],
        };
        let result = parse_function_call(syn::parse_quote!(function1(1, test = 2.4))).unwrap();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_parse_function_arguments() {
        let expected = vec![FunctionArgument {
            ident: Some("x".to_string()),
            value: ParsedValue::ParsedInt(1),
        }];
        let result = parse_function_arguments(syn::parse_quote!(x = 1)).unwrap();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_parse_value() {
        let expected = ParsedValue::ParsedPath("std :: mem :: replace".to_string());
        let result = parse_value(syn::parse_quote!(std::mem::replace)).unwrap();
        assert_eq!(expected, result);

        let expected = ParsedValue::ParsedInt(1);
        let result = parse_value(syn::parse_quote!(1)).unwrap();
        assert_eq!(expected, result);

        let expected = ParsedValue::ParseFloat(1.5);
        let result = parse_value(syn::parse_quote!(1.5)).unwrap();
        assert_eq!(expected, result);

        let expected = ParsedValue::ParsedBool(false);
        let result = parse_value(syn::parse_quote!(false)).unwrap();
        assert_eq!(expected, result);

        let expected = ParsedValue::ParsedString("test".to_string());
        let result = parse_value(syn::parse_quote!("test")).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    #[should_panic]
    fn test_parse_incorrect_value() {
        parse_value(syn::parse_quote!(let sd = s)).unwrap();
    }
}
