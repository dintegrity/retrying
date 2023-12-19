use crate::RetryingContext;

pub trait Stop {
    fn stop_execution(&self, ctx: &RetryingContext) -> bool;
}

pub struct StopAttempts {
    attempts: u32,
}

impl StopAttempts {
    pub fn new(attempts: u32) -> StopAttempts {
        StopAttempts { attempts }
    }
}

impl Stop for StopAttempts {
    fn stop_execution(&self, ctx: &RetryingContext) -> bool {
        ctx.attempt_num >= self.attempts
    }
}

pub struct StopDuration {
    duration: f32,
}

impl StopDuration {
    pub fn new(duration: f32) -> StopDuration {
        StopDuration { duration }
    }
}

impl Stop for StopDuration {
    fn stop_execution(&self, ctx: &RetryingContext) -> bool {
        ::std::time::SystemTime::now()
            .duration_since(ctx.started_at())
            .unwrap()
            .as_secs_f32()
            >= self.duration
    }
}

pub struct StopAttemptsOrDuration {
    attempts: StopAttempts,
    duration: StopDuration,
}

impl StopAttemptsOrDuration {
    pub fn new(attempts: u32, duration: f32) -> StopAttemptsOrDuration {
        StopAttemptsOrDuration {
            attempts: StopAttempts { attempts },
            duration: StopDuration { duration },
        }
    }
}

impl Stop for StopAttemptsOrDuration {
    fn stop_execution(&self, ctx: &RetryingContext) -> bool {
        self.attempts.stop_execution(ctx) || self.duration.stop_execution(ctx)
    }
}

mod tests {

    #[test]
    fn test_attemps_stop_execution() {
        use super::*;
        use crate::stop::Stop;

        let stop = StopAttempts { attempts: 2u32 };
        let mut ctx = RetryingContext::default();

        assert!(!stop.stop_execution(&ctx));

        ctx.add_attempt();
        assert!(stop.stop_execution(&ctx));
    }

    #[test]
    fn test_duration_stop_execution() {
        use super::*;
        use crate::*;

        let ctx = RetryingContext::default();
        let stop = StopDuration { duration: 0.1f32 };

        sleep_sync(Duration::from_secs_f32(0.2f32));
        assert!(stop.stop_execution(&ctx));
    }
}
