use std::str::FromStr;
use proc_macro2::TokenStream;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use crate::errors::RetryConfigurationError;

// In syn 2.0 AttributeArgs was removed, so now we can use type alias to simplify syntaxis
type AttributeArgs = syn::punctuated::Punctuated<syn::Meta, syn::Token![,]>;


pub enum WaitConfig {
    Fixed { seconds: u32 },
    Random { min: u32, max: u32 },
    Exponential { multiplier: u32, min: u32, max: u32, exp_base: u32},
}

pub(crate) struct  StopConfig {
    pub(crate) attempts: Option<u32>,
    pub(crate) duration: Option<u32>
}

pub(crate) struct RetryConfig {
    pub(crate) stop: Option<StopConfig>,
    pub(crate) wait: Option<WaitConfig>,
    pub(crate) envs_prefix: Option<String>,
}

impl RetryConfig {

    fn new() -> RetryConfig {
        RetryConfig{
            stop: None,
            wait: None,
            envs_prefix: None
        }
    }

    fn stop(&mut self, expr: syn::Expr) -> Result<(), RetryConfigurationError> {
        let mut attempts = None;
        let mut duration = None;
        let functions = parse_functions_expr(expr)?;
    
        for func in functions {
            match func.ident.as_str() {
                "attempts" => attempts = func.args.first().map(|arg|arg.value.to_string().parse::<u32>().unwrap()),
                "duration" => duration = func.args.first().map(|arg|arg.value.to_string().parse::<u32>().unwrap()),
                unknown => return Err(RetryConfigurationError::from_string(format!("Configuration {} is wrong for stop. Possible configuration option is `attempts` and `duration`", unknown)))
            }
        }

        Ok(self.stop = Some(StopConfig { attempts, duration }))
        
    }

    fn wait(&mut self, expr: syn::Expr) -> Result<(), RetryConfigurationError> {
        let ParsedFunction{ident, args} = parse_function(expr)?;
        match ident.as_str() {
            "fixed" => {
                if args.len() > 1 || args.first().filter(|x|x.ident.is_some()).is_some() {
                    Err(RetryConfigurationError::from_str("wait=fixed has only one argument without name. For exampe, `wait=fixed(1)`"))
                } else {
                    let value = args.first().map(|x|WaitConfig::Fixed{seconds: x.value.to::<u32>().unwrap()});
                    Ok(self.wait = value)
                }            
            },
            "random" => {

                let mut min: u32 = 0;
                let mut max: u32 = 3600;

                for FunctionArgument { ident, value } in args {
                    match ident {
                        Some(x) if x == "min".to_string() => min = value.to::<u32>()?,
                        Some(x) if x == "max".to_string() => max = value.to::<u32>()?,
                        _ => return Err(RetryConfigurationError::from_str("wait=random has wrong confugiration. Only `max` and `min` attributes is possible")),
                        
                    }
                };

                Ok(self.wait = Some(WaitConfig::Random { min, max }))             
            },               
            "exponential" => {

                let mut min: u32 = 0;
                let mut max: u32 = 3600;
                let mut multiplier: u32 = 1;
                let mut exp_base: u32 = 2;

                for FunctionArgument { ident, value } in args {
                    match ident {
                        Some(x) if x == "min".to_string() => min = value.to::<u32>()?,
                        Some(x) if x == "max".to_string() => max = value.to::<u32>()?,
                        Some(x) if x == "multiplier".to_string() => multiplier = value.to::<u32>()?,
                        Some(x) if x == "exp_base".to_string() => exp_base = value.to::<u32>()?,
                        _ => return Err(RetryConfigurationError::from_str("wait=exponential has wrong configuration. Only `multiplier`, `max`, `min` and `exp_base` attributes is possible")),
                        
                    }
                };

                Ok(self.wait = Some(WaitConfig::Exponential { multiplier, min, max, exp_base }))    
            },
            unknown => Err(RetryConfigurationError::from_string(format!("Configuration {} is wrong for wait. Possible configuration is `wait_fixed`, `wait_random` and `wait_exponential`", unknown)))
        }
    }

    fn retry(&mut self, _expr: syn::Expr) -> Result<(), RetryConfigurationError> {
        unimplemented!("retry option is not implemented yet")
    }

    fn env_prefix(&mut self, expr: syn::Expr) -> Result<(), RetryConfigurationError> {
        let value = match expr {
            syn::Expr::Lit(syn::ExprLit{lit, ..}) => parse_lit(lit),
            _ => Err(RetryConfigurationError::from_str("`env_prefix` value should be string literal (for exampe `env_prefix=\"retry\"`)"))
        }?;

        self.envs_prefix = Some(value.to_string());
        Ok(())
    }

    pub(crate) fn from_token_stream(args: TokenStream) -> Result<RetryConfig, RetryConfigurationError> {

        if args.is_empty() {
            Ok(RetryConfig::new())
        } else {
            let args = AttributeArgs::parse_terminated.parse2(args)
            .or(Err(RetryConfigurationError::from_str("Can't parse comma delimeted retry configuration")))?;
        
            let mut config = RetryConfig::new();

            for arg in args {
                match arg {
                    syn::Meta::NameValue(name_value) => {
                        let ident = &name_value.path.get_ident()
                        .ok_or_else(|| RetryConfigurationError::from_str("Named value without ident"))
                        .map(|v| v.to_string().to_lowercase())?;
                        let value = name_value.value;                 
        
                        match ident.as_str()  {
                            "stop" => config.stop(value)?,
                            "wait" => config.wait(value)?,
                            "retry" => config.retry(value)?,
                            "env_prefix" => config.env_prefix(value)?,
                            unknown => return Err(RetryConfigurationError::from_string(format!("Unkownd configuration name `{}`. Possible values `stop`,`wait`, `env`.", unknown)))
                        }
                    },
                    _ => return Err(RetryConfigurationError::from_str("Unkown token. Only `name=value` is acceptable in retry config"))                
                }     
            }
            Ok(config)
        }    
    }  
}


enum ParsedLit {
    ParsedInt(usize),
    ParsedString(String),
    ParsedBool(bool),
}

impl ParsedLit {
    fn to_string(&self) -> String {
        // TODO check other ways because this is ugly, maybe we can do this simple with enums
        match self {
            ParsedLit::ParsedInt(v) => v.to_string(),
            ParsedLit::ParsedString(v) => v.to_string(),
            ParsedLit::ParsedBool(v) => v.to_string()
        }
    }

    fn to<T: FromStr>(&self) -> Result<T, RetryConfigurationError> {
        self.to_string().parse::<T>().map_err(|_|RetryConfigurationError::from_str("Failed cast literal"))
    }
}

struct FunctionArgument {
    ident: Option<String>,
    value: ParsedLit,
}

struct ParsedFunction {
    ident: String,
    args: Vec<FunctionArgument>
}

fn parse_functions_expr(functions_expr: syn::Expr) -> Result<Vec<ParsedFunction>, RetryConfigurationError> {
    match functions_expr {
        syn::Expr::Paren(syn::ExprParen{expr, ..}) => {
            let deref_expr = *expr;
            match deref_expr {
                syn::Expr::Binary(syn::ExprBinary{left, right, ..}) => {
                    let left = *left;
                    let right = *right;
                    let parsed_left = parse_function(left)?;
                    let parsed_right = parse_function(right)?;
                    Ok(vec![parsed_left, parsed_right])
                },
                syn::Expr::Call(_) => {
                    let parsed_function = parse_function(deref_expr)?;
                    Ok(vec![parsed_function])
                }
                _ => Err(RetryConfigurationError::from_str("Incorrect expression between paren. Supported only one function `function(args)` or multiple functions `function(args) ||function2(args)"))
            }
        },
        _ => {
            let parsed_function = parse_function(functions_expr)?;
            Ok(vec![parsed_function])

        }
    }
}

 
fn parse_function(function_expr: syn::Expr) -> Result<ParsedFunction, RetryConfigurationError> {
    match function_expr {
        syn::Expr::Call(syn::ExprCall{func, args, ..}) => match *func {
            syn::Expr::Path(syn::ExprPath{path, ..}) => match path.get_ident().map(|x|x.to_string()) {
                Some(ident) => {
                    let args: Vec<FunctionArgument> = parse_function_arguments(args)?;
                    Ok(ParsedFunction{ident, args})
                },
                None => Err(RetryConfigurationError::from_str("Incorrect function ident"))
            },
            _ => unimplemented!() 
        },
        _ => Err(RetryConfigurationError::from_str("Not a function"))
    }
}


fn parse_function_arguments(arguments: Punctuated<syn::Expr, syn::Token![,]>) -> Result<Vec<FunctionArgument>, RetryConfigurationError> {

    let mut parsed_arguments: Vec<FunctionArgument> = Vec::new();

    for arg in arguments {
        match arg {
            // Parse name value attribute
            syn::Expr::Assign(syn::ExprAssign{left, right, ..}) => {
                let name = parse_path(*left)?;
                let value = *right;
                match value {
                    syn::Expr::Lit(syn::ExprLit{lit, ..}) => {
                        let parsed_value = parse_lit(lit)?;
                        parsed_arguments.push(FunctionArgument{ident: Some(name), value: parsed_value});
                    },
                    _ =>  return Err(RetryConfigurationError::from_str("Only literal supported in function arguments value"))
                }            
            },
            // Parse pure literal like int or string
            syn::Expr::Lit(syn::ExprLit{lit, ..}) => {
                let parsed_value = parse_lit(lit)?;
                parsed_arguments.push(FunctionArgument{ident: None, value: parsed_value});
            },
            _ => return Err(RetryConfigurationError::from_str("Incorrect function argument. Possible values are literals(`1` or \"str\") or assignemnt(x = 1, x = \"str\")"))
        }
    }
    Ok(parsed_arguments)  
}


fn parse_path(expr: syn::Expr) -> Result<String, RetryConfigurationError> {
    match expr {
        syn::Expr::Path(syn::ExprPath{path, ..}) => path
        .get_ident()
        .ok_or_else(|| RetryConfigurationError::from_str("Named value without ident"))
        .map(|v| v.to_string().to_lowercase()),
        _ =>  Err(RetryConfigurationError::from_str("Incorrec parse_path"))
    } 
}

fn parse_lit(lit: syn::Lit) -> Result<ParsedLit, RetryConfigurationError> {
    match lit {
        syn::Lit::Int(lit) => match lit.base10_parse::<usize>() {
            Ok(value) => Ok(ParsedLit::ParsedInt(value)),
            Err(e) => Err(RetryConfigurationError::from_string(format!("Failed to parse LitInt to usize. Error: {}", e))),
        },
        syn::Lit::Str(s) => Ok(ParsedLit::ParsedString(s.value())),
        syn::Lit::Verbatim(s) => Ok(ParsedLit::ParsedString(s.to_string())),
        syn::Lit::Bool(b) => Ok(ParsedLit::ParsedBool(b.value)),
        _ => Err(RetryConfigurationError::from_str("Unsupported literal. Currently supported only Int, Str and Verbatim")),
    }
}

