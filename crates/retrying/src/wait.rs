use crate::RetryingContext;

pub trait Wait {
    fn wait_seconds(&self, ctx: &RetryingContext) -> f32;

    fn wait_duration(&self, ctx: &RetryingContext) -> crate::Duration {
        crate::Duration::from_secs_f32(self.wait_seconds(ctx))
    }
}

pub struct WaitFixed {
    seconds: f32,
}

impl WaitFixed {
    pub fn new(seconds: f32) -> WaitFixed {
        WaitFixed { seconds }
    }
}

impl Wait for WaitFixed {
    fn wait_seconds(&self, _ctx: &RetryingContext) -> f32 {
        self.seconds
    }
}

pub struct WaitRandom {
    min: f32,
    max: f32,
}

impl WaitRandom {
    pub fn new(min: f32, max: f32) -> WaitRandom {
        WaitRandom { min, max }
    }
}

impl Wait for WaitRandom {
    fn wait_seconds(&self, _ctx: &RetryingContext) -> f32 {
        use crate::rand::Rng;
        let mut random_rng = crate::rand::thread_rng();
        random_rng.gen_range(self.min..=self.max) as f32
    }
}

pub struct WaitExponential {
    multiplier: f32,
    min: f32,
    max: f32,
    exp_base: u32,
}

impl WaitExponential {
    pub fn new(multiplier: f32, min: f32, max: f32, exp_base: u32) -> WaitExponential {
        WaitExponential {
            multiplier,
            min,
            max,
            exp_base,
        }
    }
}

impl Wait for WaitExponential {
    fn wait_seconds(&self, ctx: &RetryingContext) -> f32 {
        self.max
            .min(self.multiplier * (self.exp_base.pow(ctx.attempt_num - 1) as f32) + self.min)
    }
}
