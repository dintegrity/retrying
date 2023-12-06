use std::num::ParseIntError;
use retrying::retry;

fn main() {
    println!("try_retry_attempts");
    try_retry_attempts("ts");
    println!("try_retry_duration");
    try_retry_duration("ts");
    println!("try_retry_attempts_fixed");
    try_retry_attempts_fixed("ts");
    println!("try_retry_attempts_random");
    try_retry_attempts_random("ts");
    println!("try_retry_attempts_random");
    try_retry_attempts_exponential("ts");
    // try_retry_with_env("ts");
}

#[retry(stop=attempts(2))]
fn try_retry_attempts(in_param: &str) -> Result<i32, ParseIntError> {
    in_param.parse::<i32>()
}

#[retry(stop=duration(1))]
fn try_retry_duration(in_param: &str) -> Result<i32, ParseIntError> {
    in_param.parse::<i32>()
}

#[retry(stop=(attempts(4)||duration(2)),wait=fixed(1))]
fn try_retry_attempts_fixed(in_param: &str) -> Result<i32, ParseIntError> {
    in_param.parse::<i32>()
}

#[retry(stop=attempts(4),wait=random(min=1,max=2))]
fn try_retry_attempts_random(in_param: &str) -> Result<i32, ParseIntError> {
    in_param.parse::<i32>()
}

#[retry(stop=attempts(4),wait=exponential(min=1,max=2))]
fn try_retry_attempts_exponential(in_param: &str) -> Result<i32, ParseIntError> {
    in_param.parse::<i32>()
}
// DEBUG result of code gen `cargo rustc --bin retrying -- -Zunpretty=expanded`
