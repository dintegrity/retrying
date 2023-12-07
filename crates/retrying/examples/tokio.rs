use std::{num::ParseIntError, thread::Thread};
use retrying::retry;

#[tokio::main]
async fn main() {   

    let mut handles = vec![];

    handles.push(tokio::spawn(
        async {
            try_retry_attempts("try_retry_attempts").await;
        }
    ));
    
    handles.push(tokio::spawn(
        async {
            try_retry_duration("try_retry_duration").await;
        }
    )); 
    
    handles.push(tokio::spawn(
        async {
            try_retry_attempts_fixed("try_retry_attempts_fixed").await;
        }
    ));
    handles.push(tokio::spawn(
        async {
            try_retry_attempts_random("try_retry_attempts_random").await;
        }
    ));    
    handles.push(tokio::spawn(
        async {
            try_retry_attempts_exponential("try_retry_attempts_exponential").await;
        }
    ));
  
    for future in handles {
        let result = handle.await;
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

#[retry(stop=(attempts(4)||duration(2)),wait=fixed(1))]
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
