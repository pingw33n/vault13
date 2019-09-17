use enum_map_derive::Enum;
use if_chain::if_chain;
use std::time::{Duration, Instant};

use crate::asset::CritterAnim;
use crate::game::object::Handle;
use crate::game::world::World;
use crate::sequence::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum State {
    Started,
    Running(Instant),
    Done,
}

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq)]
pub enum AnimDirection {
    Forward,
    Backward,
}

pub struct FrameAnim {
    obj: Handle,
    anim: Option<CritterAnim>,
    direction: AnimDirection,
    /// If true makes the animation loop forever.
    wrap: bool,
    frame_len: Duration,
    state: State,
}

impl FrameAnim {
    pub fn new(obj: Handle, anim: Option<CritterAnim>, direction: AnimDirection,
        wrap: bool) -> Self
    {
        Self {
            obj,
            anim,
            direction,
            wrap,
            frame_len: Duration::from_millis(1000 / 10),
            state: State::Started,
        }
    }

    fn init(&mut self, world: &mut World) {
        let mut obj = world.objects().get(self.obj).borrow_mut();

        obj.fid = if_chain! {
            if let Some(anim) = self.anim;
            if let Some(fid) = obj.fid.critter();
            then {
                fid.with_anim(anim).into()
            } else {
                obj.fid
            }
        };

        match self.direction {
            AnimDirection::Forward => obj.frame_idx = 0,
            AnimDirection::Backward => {
                let frame_set = world.frm_db().get(obj.fid).unwrap();
                let frames = &frame_set.frame_lists[obj.direction].frames;
                obj.frame_idx = frames.len() - 1;
            }
        }

        self.frame_len = Duration::from_millis(1000 / world.frm_db().get(obj.fid).unwrap().fps as u64);
    }
}

impl Sequence for FrameAnim {
    fn update(&mut self, ctx: &mut Update) -> Result {
        match self.state {
            State::Started => {
                self.init(ctx.world);
            },
            State::Running(last_time) => {
                if ctx.time - last_time < self.frame_len {
                    return Result::Running(Running::NotLagging);
                }
            }
            State::Done => return Result::Done,
        }

        let shift = {
            let mut obj = ctx.world.objects().get(self.obj).borrow_mut();

            let frame_set = ctx.world.frm_db().get(obj.fid).unwrap();
            let frames = &frame_set.frame_lists[obj.direction].frames;

            if self.state != State::Started {
                match self.direction {
                    AnimDirection::Forward => {
                        let done = if obj.frame_idx + 1 < frames.len() {
                            obj.frame_idx += 1;
                            false
                        } else if self.wrap {
                            obj.frame_idx = 0;
                            false
                        } else {
                            true
                        };
                        if done {
                            None
                        } else {
                            Some(frames[obj.frame_idx].shift)
                        }
                    }
                    AnimDirection::Backward => {
                        let done = if obj.frame_idx > 0 {
                            obj.frame_idx -= 1;
                            false
                        } else if self.wrap {
                            obj.frame_idx = frames.len() - 1;
                            false
                        } else {
                            true
                        };
                        if done {
                            None
                        } else {
                            Some(-frames[obj.frame_idx + 1].shift)
                        }
                    }
                }
            } else {
                Some(frames[obj.frame_idx].shift)
            }
        };
        if let Some(shift) = shift {
            ctx.world.objects_mut().add_screen_shift(self.obj, shift);
        } else {
            self.state = State::Done;
            return Result::Running(Running::NotLagging);
        }

        let new_last_time = if let State::Running(last_time) = self.state {
            last_time + self.frame_len
        } else {
            ctx.time
        };
        self.state = State::Running(new_last_time);

        Result::Running(if ctx.time - new_last_time < self.frame_len {
            Running::NotLagging
        } else {
            Running::Lagging
        })
    }
}