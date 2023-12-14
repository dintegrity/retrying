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
