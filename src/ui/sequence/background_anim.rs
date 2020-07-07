use std::time::{Duration, Instant};

use crate::sequence::*;
use super::super::Handle;

#[derive(Clone, Copy)]
enum State {
    Init,
    Running {
        last: Instant,
    },
}

pub struct BackgroundAnim {
    widget: Handle,
    delay: Duration,
    state: State,
}

impl BackgroundAnim {
    pub fn new(widget: Handle, delay: Duration) -> Self {
        Self {
            widget,
            delay,
            state: State::Init,
        }
    }
}

impl Sequence for BackgroundAnim {
    fn update(&mut self, ctx: &mut Update) -> Result {
        match self.state {
            State::Init => {
                self.state = State::Running {
                    last: ctx.time,
                };
                Result::Running(Running::NotLagging)
            }
            State::Running { last } => {
                let elapsed = ctx.time - last;
                if elapsed >= self.delay {
                    let mut base = ctx.ui.widget_base_mut(self.widget);
                    if let Some(bkg) = base.background_mut() {
                        bkg.direction = bkg.direction.rotate_cw();
                    }
                    self.state = State::Running {
                        last: last + self.delay,
                    };
                }
                Result::Running(if elapsed >= self.delay * 2 {
                    Running::Lagging
                } else {
                    Running::NotLagging
                })
            }
        }
    }
}