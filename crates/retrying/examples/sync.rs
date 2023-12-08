use std::num::ParseIntError;
use retrying::retry;

#[allow(unused_must_use)]
fn main() {

   try_retry_attempts("try_retry_attempts");

   try_retry_duration("try_retry_duration");

   try_retry_attempts_fixed("try_retry_attempts_fixed");

   try_retry_attempts_random("try_retry_attempts_random");

   try_retry_attempts_exponential("try_retry_attempts_exponential");

   std::env::set_var("RETRYING_TEST__STOP__ATTEMPTS", "3");
   std::env::set_var("RETRYING_TEST__WAIT__FIXED", "1.01");
   
   try_retry_attempts_fixed_env("try_retry_attempts_fixed_env");

   try_retry("try_retry");

}


#[retry(stop=attempts(2))]
fn try_retry_attempts(in_param: &str) -> Result<i32, ParseIntError> {
    println!("{}", in_param);
    in_param.parse::<i32>()
}

#[retry(stop=duration(0.2))]
fn try_retry_duration(in_param: &str) -> Result<i32, ParseIntError> {
    println!("{}", in_param);
    in_param.parse::<i32>()
}

#[retry(stop=(attempts(4)||duration(2)),wait=fixed(1))]
fn try_retry_attempts_fixed(in_param: &str) -> Result<i32, ParseIntError> {
    println!("{}", in_param);
    in_param.parse::<i32>()
}

#[retry(stop=attempts(4),wait=random(min=1,max=1.555))]
fn try_retry_attempts_random(in_param: &str) -> Result<i32, ParseIntError> {
    println!("{}", in_param);
    in_param.parse::<i32>()
}

#[retry(stop=attempts(4),wait=exponential(multiplier=0.555, min=1,max=10))]
fn try_retry_attempts_exponential(in_param: &str) -> Result<i32, ParseIntError> {
    println!("{}", in_param);
    in_param.parse::<i32>()
}

#[retry(stop=attempts(1000),wait=fixed(1000.4),env_prefix="RETRYING_TEST")]
fn try_retry_attempts_fixed_env(in_param: &str) -> Result<i32, ParseIntError> {
    println!("{}", in_param);
    in_param.parse::<i32>()
}

#[retry]
fn try_retry(in_param: &str) -> Result<i32, ParseIntError> {
    println!("retry macros without parameters will never stop. Use CTRL+C to top this example");
    ::retrying::sleep_sync(::retrying::Duration::from_secs(2));
    in_param.parse::<i32>()
}

