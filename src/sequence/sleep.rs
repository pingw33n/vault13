use std::time::{Duration, Instant};

use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum State {
    Started,
    Running(Instant),
}

pub struct Sleep {
    state: State,
    duration: Duration,
}

impl Sleep {
    pub fn new(duration: Duration) -> Self {
        Self {
            state: State::Started,
            duration,
        }
    }
}

impl Sequence for Sleep {
    fn update(&mut self, ctx: &mut Context) -> Result {
        match self.state {
            State::Started => self.state = State::Running(ctx.time),
            State::Running(start_time) => {
                if ctx.time - start_time >= self.duration {
                    return Result::Done(Done::AdvanceLater);
                }
            }
        }
        Result::Running(Running::NotLagging)
    }
}