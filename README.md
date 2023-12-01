# tenacity-rs

Rust implementation of python [tenacity](https://github.com/jd/tenacity) library.

### Configuration option
* #### Stop options
| Config option | OS Environments | Default | Description|
|:---|:---|:---|:---|
| stop=stop_after_attempt(`u32`) | {PREFIX}__STOP__STOP_AFTER_ATTEMPT | std::usize::MAX | Number of retries|
| stop=stop_after_delay(`u32`) | {PREFIX}__STOP__STOP_AFTER_DELAY | std::usize::MAX | Retrying period (seconds) ||

You can combine several stop conditions by using the or(`|`) operator. For example, configuration  
```rust
#[tenacity::retry(stop=(stop_after_attempt(10)|stop_after_delay(60)))]
fn my_function(){}
```
means retry 10 times but not make new attemp after 60 seconds.


* #### Wait options
| Config option | OS Environments | Default | Description |
| :--- | :--- | :--- | :--- |
| wait=wait_fixed(`u32`) | {PREFIX}__WAIT__WAIT_FIXED | 0 | Number of seconds between retries |
| wait=wait_random(min=`u32`,max=`u32`) | {PREFIX}__WAIT__WAIT_RANDOM\__(MIN\|MAX) | min=0,max=3600 | Randomly wait _min_ to _max_ seconds between retries |
| wait=wait_exponential(multiplier=`u32`, min=`u32`,max=`u32`,exp_base=`u32`) | {PREFIX}__WAIT__WAIT_EXPONENTIAL\__(MULTIPLIER\|MIN\|MAX\|EXP_BASE) | multiplier=1, min=0,max=3600, exp_base=2 | Wait _multiplier_ * _exp_base_^(num of retry - 1) + _min_ seconds between each retry starting with _min_ seconds, then up to _max_ seconds, then _max_ seconds afterwards |

Only one wait option possible.

* #### Retry option
:warning: NOT IMPLEMENTED. Will be supported in future releases

| Config option | OS Environments | Default | Description |
| :--- | :--- | :--- | :--- |
| retry=retry_if_exception(exception1\|exception2) | {PREFIX}__RETRY__RETRY_IF_EXCEPTION | - | Retry only on specific exeptions |
| retry=retry_if_not_exception(exception1\|exception2) | {PREFIX}__RETRY__RETRY_IF_NOT_EXCEPTION | - | Don't retry on specific list of exeptions |

### Using OS environment variables for retry configuration
:warning: Code generation is not implemented for this feature. So, event if option is specified the macros ignores it. Will be supported in future releases
Tenacity-rs allow to override macros configuration in runtime using env variables. To enable this feature you need to specify prefix for configuration env variable like
```
#[tenacity::retry(env_prefix="test")]
```
Also, it is possible to combine macros configuration option with `env_prefix`` but take into account that OS environment variables has higher priority than options configuration in code.
For example,
```
#[tenacity::retry(stop_after_attempt(2), env_prefix="test")]
```
In runtime macros will check availability of OS env variable TEST__STOP__STOP_AFTER_ATTEMPT and if variable is set then number of retry attempt will be the value of TEST__STOP__STOP_AFTER_ATTEMPT.