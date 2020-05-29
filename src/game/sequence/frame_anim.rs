use enum_map_derive::Enum;
use if_chain::if_chain;
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
        let mut obj = world.objects().get_mut(self.obj);

        obj.fid = if_chain! {
            if let Some(anim) = self.anim;
            if let Some(fid) = obj.fid.critter();
            then {
                fid.with_anim(anim).into()
            } else {
                obj.fid
            }
        };

        self.frame_len = Duration::from_millis(1000 / world.frm_db().get(obj.fid).unwrap().fps as u64);
    }
}

impl Sequence for FrameAnim {
    fn update(&mut self, ctx: &mut Update) -> Result {
        let set_frame = match self.state {
            State::Started => {
                self.init(ctx.world);
                match self.direction {
                    AnimDirection::Forward => SetFrame::Index(0),
                    AnimDirection::Backward => SetFrame::Last,
                }
            },
            State::Running(last_time) => {
                if ctx.time - last_time < self.frame_len {
                    return Result::Running(Running::NotLagging);
                }

                let frame_index = {
                    let mut obj = ctx.world.objects().get_mut(self.obj);

                    let frame_set = ctx.world.frm_db().get(obj.fid).unwrap();
                    let frames = &frame_set.frame_lists[obj.direction].frames;

                    let done = match self.direction {
                        AnimDirection::Forward => {
                            if obj.frame_idx + 1 < frames.len() {
                                obj.frame_idx += 1;
                                false
                            } else if self.wrap {
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
                            } else if self.wrap {
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