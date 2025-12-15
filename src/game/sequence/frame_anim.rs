use linearize::Linearize;
use std::time::{Duration, Instant};

use crate::asset::CritterAnim;
use crate::game::object::{Handle, SetFrame};
use crate::game::world::World;
use crate::sequence::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum State {
    Started,
    Running(Instant),
    Done,
}

#[derive(Clone, Copy, Debug, Linearize, Eq, PartialEq)]
pub enum AnimDirection {
    Forward,
    Backward,
}

#[derive(Clone, Debug)]
pub struct FrameAnimOptions {
    pub anim: Option<CritterAnim>,
    pub direction: AnimDirection,

    /// If `true` makes the animation loop forever.
    pub wrap: bool,

    /// Number of frames to skip initially.
    pub skip: u32,
}

impl Default for FrameAnimOptions {
    fn default() -> Self {
        Self {
            anim: None,
            direction: AnimDirection::Forward,
            wrap: false,
            skip: 0,
        }
    }
}

pub struct FrameAnim {
    obj: Handle,
    options: FrameAnimOptions,
    frame_len: Duration,
    state: State,
}

impl FrameAnim {
    pub fn new(obj: Handle, options: FrameAnimOptions) -> Self {
        Self {
            obj,
            options,
            frame_len: Duration::from_millis(1000 / 10),
            state: State::Started,
        }
    }

    fn init(&mut self, world: &mut World) {
        let mut obj = world.objects().get_mut(self.obj);

        obj.fid = if let Some(anim) = self.options.anim &&
            let Some(fid) = obj.fid.critter()
        {
            fid.with_anim(anim).into()
        } else {
            obj.fid
        };

        self.frame_len = Duration::from_millis(1000 / world.frm_db().get(obj.fid).unwrap().fps as u64);
    }
}

impl Sequence for FrameAnim {
    fn update(&mut self, ctx: &mut Update) -> Result {
        let set_frame = match self.state {
            State::Started => {
                self.init(ctx.world);
                SetFrame::Index(match self.options.direction {
                    AnimDirection::Forward => self.options.skip as usize,
                    AnimDirection::Backward => {
                        let obj = ctx.world.objects().get(self.obj);
                        let frame_set = ctx.world.frm_db().get(obj.fid).unwrap();
                        let frames = &frame_set.frame_lists[obj.direction].frames;
                        frames.len().checked_sub(1 + self.options.skip as usize).unwrap()
                    }
                })
            },
            State::Running(last_time) => {
                if ctx.time - last_time < self.frame_len {
                    return Result::Running(Running::NotLagging);
                }

                let frame_index = {
                    let mut obj = ctx.world.objects().get_mut(self.obj);

                    let frame_set = ctx.world.frm_db().get(obj.fid).unwrap();
                    let frames = &frame_set.frame_lists[obj.direction].frames;

                    let done = match self.options.direction {
                        AnimDirection::Forward => {
                            if obj.frame_idx + 1 < frames.len() {
                                obj.frame_idx += 1;
                                false
                            } else if self.options.wrap {
                                obj.frame_idx = 0;
                                false
                            } else {
                                true
                            }
                        }
                        AnimDirection::Backward => {
                            if obj.frame_idx > 0 {
                                obj.frame_idx -= 1;
                                false
                            } else if self.options.wrap {
                                obj.frame_idx = frames.len() - 1;
                                false
                            } else {
                                true
                            }
                        }
                    };
                    if done {
                        None
                    } else {
                        Some(obj.frame_idx)
                    }
                };
                if let Some(frame_index) = frame_index {
                    SetFrame::Index(frame_index)
                } else {
                    self.state = State::Done;
                    return Result::Running(Running::NotLagging);
                }
            }
            State::Done => return Result::Done,
        };

        ctx.world.objects_mut().set_frame(self.obj, set_frame);

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