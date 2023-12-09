use retrying::retry;
use std::num::ParseIntError;

fn main() {
    let mut handles = vec![];

    handles.push(async_std::task::spawn(async {
        let _ = try_retry_attempts("try_retry_attempts").await;
    }));

    handles.push(async_std::task::spawn(async {
        let _ = try_retry_duration("try_retry_duration").await;
    }));

    handles.push(async_std::task::spawn(async {
        let _ = try_retry_attempts_fixed("try_retry_attempts_fixed").await;
    }));
    handles.push(async_std::task::spawn(async {
        let _ = try_retry_attempts_random("try_retry_attempts_random").await;
    }));
    handles.push(async_std::task::spawn(async {
        let _ = try_retry_attempts_exponential("try_retry_attempts_exponential").await;
    }));

    std::env::set_var("MY_METHOD__RETRYING__STOP__ATTEMPTS", "3");
    std::env::set_var("MY_METHOD__RETRYING__WAIT__FIXED", "2");
    handles.push(async_std::task::spawn(async {
        let _ = try_retry_attempts_fixed_env("try_retry_attempts_fixed_env").await;
    }));

    for future in handles {
        async_std::task::block_on(future)
    }
}

#[retry(stop=attempts(2))]
async fn try_retry_attempts(in_param: &str) -> Result<i32, ParseIntError> {
    println!("{}", in_param);
    in_param.parse::<i32>()
}

#[retry(stop=duration(1))]
async fn try_retry_duration(in_param: &str) -> Result<i32, ParseIntError> {
    println!("{}", in_param);
    in_param.parse::<i32>()
}

#[retry(stop=(attempts(4)|duration(2)),wait=fixed(1))]
async fn try_retry_attempts_fixed(in_param: &str) -> Result<i32, ParseIntError> {
    println!("{}", in_param);
    in_param.parse::<i32>()
}

#[retry(stop=attempts(4),wait=random(min=1,max=2))]
async fn try_retry_attempts_random(in_param: &str) -> Result<i32, ParseIntError> {
    println!("{}", in_param);
    in_param.parse::<i32>()
}

#[retry(stop=attempts(4),wait=exponential(min=1,max=10))]
async fn try_retry_attempts_exponential(in_param: &str) -> Result<i32, ParseIntError> {
    println!("{}", in_param);
    in_param.parse::<i32>()
}

#[retry(stop=attempts(1000),wait=fixed(1000),envs_prefix="MY_METHOD")]
async fn try_retry_attempts_fixed_env(in_param: &str) -> Result<i32, ParseIntError> {
    println!("{}", in_param);
    in_param.parse::<i32>()
}
