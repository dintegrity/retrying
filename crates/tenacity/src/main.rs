use std::num::ParseIntError;
use tenacity::retry;

fn main() {
    try_retry("ts").unwrap();
    // try_retry_with_env("ts");
}

#[retry(stop=(stop_after_attempt(10)|stop_after_duration(60)), wait=wait_exponential(min=2,max=12), env_prefix="test")]
fn try_retry(in_param: &str) -> Result<i32, ParseIntError> {
    in_param.parse::<i32>()
}

// DEBUG result of code gen `cargo rustc --bin tenacity -- -Zunpretty=expanded`
