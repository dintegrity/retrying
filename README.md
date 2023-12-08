# retrying

General-purpose retrying library, written in Rust, to simplify the task of adding retry behavior to rust functions.

Support sync and async ([tokio](https://tokio.rs/), [async-std](https://async.rs/)) functions.

## Macros

The main public interface in retrying is ```retrying::retry``` macros.

```rust
#[retrying::retry]
fn my_function(){}
```
This macros has a lot of configuration options and allows developers to write fault tolerant functions without thinking about implementation of retry functionality.
:warning: The macros generates code and adds variables with prefix `retrying_` into code. To avoid variable conflicts please don't use variables with this prefix in functions with `retry` macros.

## Configuration option
* ### Stop

This section describes configuration options that specify when method execution should stop retrying.

| Config option | OS Environments | Default | Description|
|:---|:---|:---|:---|
| stop=attempts(`u32`) | {PREFIX}__STOP__ATTEMPTS | std::u32::MAX | Number of retries|
| stop=delay(`u32`) | {PREFIX}__STOP__DELAY | std::u32::MAX | Retrying period (seconds) ||

It is possible to combine several _stop_ conditions per method by using the or(`||`) operator. For example, configuration  
```rust
#[retrying::retry(stop=(attempts(10)||delay(60)))]
fn my_function(){}
```
means the function should retry 10 times but doesn't make new attempt after 60 seconds.


* ### Wait

This section describes configuration options that specify delay between each attempt.

| Config option | OS Environments | Default | Description |
| :--- | :--- | :--- | :--- |
| wait=fixed(`u32`) | {PREFIX}__WAIT__FIXED | 0 | Number of seconds between retries |
| wait=random(min=`u32`, max=`u32`) | {PREFIX}__WAIT__RANDOM\__(MIN\|MAX) | min=0,max=3600 | Randomly wait _min_ to _max_ seconds between retries |
| wait=exponential(multiplier=`u32`, min=`u32`, max=`u32`, exp_base=`u32`) | {PREFIX}__WAIT__EXPONENTIAL\__(MULTIPLIER\|MIN\|MAX\|EXP_BASE) | multiplier=1, min=0, max=3600, exp_base=2 | Wait _multiplier_ * _exp_base_^(num of retry - 1) + _min_ seconds between each retry starting with _min_ seconds, then up to _max_ seconds, then _max_ seconds afterwards |

Only one _wait_ option possible per method.

* ### Retry
:warning: NOT IMPLEMENTED. Will be supported in future releases

This section describes configuration options that specify retrying conditions.

| Config option | OS Environments | Default | Description |
| :--- | :--- | :--- | :--- |
| retry=if_exception(exception1\|\|exception2\|\|exception2) | {PREFIX}__RETRY__RETRY_IF_EXCEPTION | - | Retry only on specific exceptions |
| retry=if_not_exception(exception1\|\|exception2\|\|exception3) | {PREFIX}__RETRY__RETRY_IF_NOT_EXCEPTION | - | Don't retry on specific exceptions |

Only one _retry_ option possible per method.

## Using OS environment variables for updating retry configuration
There are certain list of use cases when retry configuration requires updating configuration values in runtime. For example, It is useful when we need a different number of attempts per environment (dev, prod, stage), systems, unit tests etc.  

Retrying allows overriding macros configuration in runtime using env variables using special configuration option `env_prefix` like  
```
#[retrying::retry(<retry configurations>,env_prefix="test")]
```
:warning: Limitations
* It is possible to override only configuration value, not configuration option. It means, for example, if configuration option `stop=attempts(1))` is not defined in macros code then the OS env variable `{PREFIX}__STOP__ATTEMPTS` doesn't affect code execution. In other words, the OS environment variable can override only the value of the configured option and it is not able to change the option itself.  
* Configuration option from the OS environment variable has a higher priority than options in source code.
* If OS environment variables are not set then macros uses the value from its configuration (source code).
* If OS environment variable has the wrong format (for example, non-numeric value is specified for numeric configuration) then retrying macros ignores such configuration, logs error in stderr and continues using values from code.

Example of usage:
```rust
#[retrying::retry(stop=attempts(2), env_prefix="test")]
```
With above configuration macros checks in runtime the availability of OS env variable TEST__STOP__ATTEMPTS (case-insensitive) and if variable is set then number of retry attempt will be the value of TEST__STOP__ATTEMPTS. If the list of OS environment contains more than one configuration option with the same prefix then macros ignores OS env variable and take configuration value from code.

## Features
tokio - builds retrying library for using with tokio asynchronous runtime.
async_std - builds retrying library for using with async_std asynchronous runtime.

## Examples
Examples are available in ./crates/retrying/example and can be tested using cargo.
Sync:
```bash
cargo run --example sync
```
Async tokio:
```bash
cargo run --features="tokio" --example tokio
```
Async async-std:
```bash
cargo run --features="async_std" --example async_std
```
