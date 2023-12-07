use std::num::ParseIntError;
use retrying::retry;

#[allow(unused_must_use)]
fn main() {

    try_retry_attempts("try_retry_attempts");

    try_retry_duration("try_retry_duration");

    try_retry_attempts_fixed("try_retry_attempts_fixed");

    try_retry_attempts_random("try_retry_attempts_random");

    try_retry_attempts_exponential("try_retry_attempts_exponential");
}

#[retry(stop=attempts(2))]
fn try_retry_attempts(in_param: &str) -> Result<i32, ParseIntError> {
    println!("{}", in_param);
    in_param.parse::<i32>()
}

#[retry(stop=duration(1))]
fn try_retry_duration(in_param: &str) -> Result<i32, ParseIntError> {
    println!("{}", in_param);
    in_param.parse::<i32>()
}

#[retry(stop=(attempts(4)||duration(2)),wait=fixed(1))]
fn try_retry_attempts_fixed(in_param: &str) -> Result<i32, ParseIntError> {
    println!("{}", in_param);
    in_param.parse::<i32>()
}

#[retry(stop=attempts(4),wait=random(min=1,max=2))]
fn try_retry_attempts_random(in_param: &str) -> Result<i32, ParseIntError> {
    println!("{}", in_param);
    in_param.parse::<i32>()
}

#[retry(stop=attempts(4),wait=exponential(min=1,max=10))]
fn try_retry_attempts_exponential(in_param: &str) -> Result<i32, ParseIntError> {
    println!("{}", in_param);
    in_param.parse::<i32>()
}

