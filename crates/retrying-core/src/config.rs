use crate::errors::RetryConfigurationError;
use proc_macro2::TokenStream;
use std::fmt;
use std::str::FromStr;
use syn::parse::Parser;
use syn::punctuated::Punctuated;

// In syn 2.0 AttributeArgs was removed, so now we can use type alias to simplify syntaxis
type AttributeArgs = syn::punctuated::Punctuated<syn::Meta, syn::Token![,]>;

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

pub(crate) struct StopConfig {
    pub(crate) attempts: Option<u32>,
    pub(crate) duration: Option<f32>,
}
impl StopConfig {
    const ATTEMPTS: &'static str = "attempts";
    const DURATION: &'static str = "duration";
}

pub(crate) struct RetryConfig {
    pub(crate) if_errors: Option<Vec<syn::Path>>,
    pub(crate) if_not_errors: Option<Vec<syn::Path>>,
}

impl RetryConfig {
    const IF_ERRORS: &'static str = "if_errors";
    const IF_NOT_ERRORS: &'static str = "if_not_errors";
}

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
        if attempts.is_some() || duration.is_some() {
            self.stop = Some(StopConfig { attempts, duration });
        }
        Ok(())
    }

    fn wait(&mut self, expr: syn::Expr) -> Result<(), RetryConfigurationError> {
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
                    self.wait = args.first().map(|x| WaitConfig::Fixed {
                        seconds: x.value.as_literal::<f32>().unwrap(),
                    });
                    Ok(())
                }
            }
            WaitConfig::RANDOM => {
                let mut min: f32 = 0.0;
                let mut max: f32 = 3600.0;

                for FunctionArgument { ident, value } in args {
                    match ident.unwrap_or(String::new()).as_str() {
                        WaitConfig::MIN => min = value.as_literal::<f32>()?,
                        WaitConfig::MAX => max = value.as_literal::<f32>()?,
                        unknown => return Err(RetryConfigurationError::new(format!("{}={} has wrong configuration {}. Only `{}` and `{}` attributes is possible", RetryingConfig::WAIT, WaitConfig::RANDOM, unknown, WaitConfig::MIN, WaitConfig::MAX))),           
                    }
                }
                self.wait = Some(WaitConfig::Random { min, max });
                Ok(())
            }
            WaitConfig::EXPONENTIAL => {
                let mut min: f32 = 0.0;
                let mut max: f32 = 3600.0;
                let mut multiplier: f32 = 1.0;
                let mut exp_base: u32 = 2;

                for FunctionArgument { ident, value } in args {
                    match ident.unwrap_or(String::new()).as_str() {
                        WaitConfig::MIN => min = value.as_literal::<f32>()?,
                        WaitConfig::MAX => max = value.as_literal::<f32>()?,
                        WaitConfig::MULTIPLIER => multiplier = value.as_literal::<f32>()?,
                        WaitConfig::EXP_BASE => exp_base = value.as_literal::<u32>()?,
                        unknown => return Err(RetryConfigurationError::new(format!("{}={} has wrong configuration option `{}`. Only `{}`, `{}`, `{}` and `{}` attributes is possible", RetryingConfig::WAIT, WaitConfig::EXPONENTIAL, unknown, WaitConfig::MIN, WaitConfig::MAX, WaitConfig::EXPONENTIAL, WaitConfig::EXP_BASE))),
                    }
                }
                self.wait = Some(WaitConfig::Exponential {
                    multiplier,
                    min,
                    max,
                    exp_base,
                });
                Ok(())
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
        let mut if_errors: Option<Vec<syn::Path>> = None;
        let mut if_not_errors: Option<Vec<syn::Path>> = None;

        let functions = parse_functions_expr(expr)?;

        for func in functions {
            if !func.args.is_empty() {
                match func.ident.as_str() {
                    RetryConfig::IF_ERRORS => if_errors = Some(func.args.iter().map(|a|a.value.as_path().unwrap()).collect()),
                    RetryConfig::IF_NOT_ERRORS => if_not_errors = Some(func.args.iter().map(|a|a.value.as_path().unwrap()).collect()),
                    unknown => return Err(RetryConfigurationError::new(format!("Configuration {} is wrong for `{}`. Possible configuration option is `{}` and `{}`", unknown, RetryingConfig::RETRY, RetryConfig::IF_ERRORS, RetryConfig::IF_NOT_ERRORS)))
                }
            }
        }

        if if_errors.is_some() && if_not_errors.is_some() {
            Err(RetryConfigurationError::new(format!("Configuration is wrong for `{}`. Only one of `{}` and `{}` should be configured at the same time", RetryingConfig::RETRY, RetryConfig::IF_ERRORS, RetryConfig::IF_NOT_ERRORS)))
        } else {
            self.retry = Some(RetryConfig {
                if_errors,
                if_not_errors,
            });
            Ok(())
        }
    }

    fn envs_prefix(&mut self, expr: syn::Expr) -> Result<(), RetryConfigurationError> {
        let value = match expr {
            syn::Expr::Lit(syn::ExprLit { lit, .. }) => parse_lit(&lit),
            _ => Err(RetryConfigurationError::new(format!(
                "`{}` value should be string literal (for exampe `envs_prefix=\"retry\"`)",
                RetryingConfig::ENVS_PREFIX
            ))),
        }?;

        self.envs_prefix = Some(value.to_string());
        Ok(())
    }

    pub(crate) fn from_token_stream(
        args: TokenStream,
    ) -> Result<RetryingConfig, RetryConfigurationError> {
        if args.is_empty() {
            Ok(RetryingConfig::new())
        } else {
            let args = AttributeArgs::parse_terminated.parse2(args).or(Err(
                RetryConfigurationError::from_str(
                    "Can't parse comma delimeted retry configuration",
                ),
            ))?;

            let mut config = RetryingConfig::new();

            for arg in args {
                match arg {
                    syn::Meta::NameValue(name_value) => {
                        let ident = &name_value
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
}

enum ParsedValue {
    ParsedInt(u32),
    ParsedString(String),
    ParsedBool(bool),
    ParseFloat(f32),
    ParsedPath(syn::Path),
}

impl fmt::Display for ParsedValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParsedValue::ParsedInt(v) => write!(f, "{v}"),
            ParsedValue::ParsedString(v) => write!(f, "{v}"),
            ParsedValue::ParsedBool(v) => write!(f, "{v}"),
            ParsedValue::ParseFloat(v) => write!(f, "{v}"),
            ParsedValue::ParsedPath(syn::Path {
                leading_colon,
                segments,
            }) => {
                let leading_colon = leading_colon.map_or("", |_| "::");
                let segments = segments
                    .iter()
                    .map(|x| x.ident.to_string())
                    .collect::<Vec<String>>()
                    .join("::");
                write!(f, "{leading_colon}{segments}")
            }
        }
    }
}

impl ParsedValue {
    fn as_path(&self) -> Result<syn::Path, RetryConfigurationError> {
        match self {
            ParsedValue::ParsedPath(x) => Ok(x.clone()),
            _ => Err(RetryConfigurationError::from_str(
                "`as_path` can be used only with ParsedValue::ParsedPath. To get value of other ParsedValues please use `as_literal` ",
            )),
        }
    }

    fn as_literal<T: FromStr>(&self) -> Result<T, RetryConfigurationError> {
        match self {
            ParsedValue::ParsedPath(_) => Err(RetryConfigurationError::from_str(
                "ParsedPath can't be casted to literal. Use `as_path` method to get syn::Path value",
            )),
            _ => self
                .to_string()
                .parse::<T>()
                .map_err(|_| RetryConfigurationError::from_str("Failed cast to literal")),
        }
    }
}

struct FunctionArgument {
    ident: Option<String>,
    value: ParsedValue,
}

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
        syn::Expr::Path(syn::ExprPath { path, .. }) => Ok(ParsedValue::ParsedPath(path)),
        syn::Expr::Lit(syn::ExprLit { lit, .. }) => parse_lit(&lit),
        _ => Err(RetryConfigurationError::from_str(
            "Incorrect value. Supported values are syn::Expr::Lit and syn::Expr::Path",
        )),
    }
}

fn parse_ident(expr: syn::Expr) -> Result<String, RetryConfigurationError> {
    match expr {
        syn::Expr::Path(syn::ExprPath { path, .. }) => path
            .get_ident()
            .ok_or_else(|| RetryConfigurationError::from_str("Named value without ident"))
            .map(|v| v.to_string().to_lowercase()),
        _ => Err(RetryConfigurationError::from_str(
            "Incorrect expression in parse_ident",
        )),
    }
}

fn parse_lit(lit: &syn::Lit) -> Result<ParsedValue, RetryConfigurationError> {
    match lit {
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
    }
}
