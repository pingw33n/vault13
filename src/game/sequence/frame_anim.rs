use enum_map_derive::Enum;
use std::time::{Duration, Instant};

use crate::asset::frm::CritterAnim;
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
    frame_len: Duration,
    state: State,
}

impl FrameAnim {
    pub fn new(obj: Handle, anim: Option<CritterAnim>, direction: AnimDirection) -> Self {
        Self {
            obj,
            anim,
            direction,
            frame_len: Duration::from_millis(1000 / 10),
            state: State::Started,
        }
    }

    fn init(&mut self, world: &mut World) {
        let mut obj = world.objects().get(&self.obj).borrow_mut();

        if let Some(anim) = self.anim {
            obj.fid = obj.fid
                .critter()
                .unwrap()
                .with_direction(Some(obj.direction))
                .with_anim(anim)
                .into()
        }

        match self.direction {
            AnimDirection::Forward => obj.frame_idx = 0,
            AnimDirection::Backward => {
                let frame_set = world.frm_db().get(obj.fid);
                let frames = &frame_set.frame_lists[obj.direction].frames;
                obj.frame_idx = frames.len() - 1;
            }
        }

        self.frame_len = Duration::from_millis(1000 / world.frm_db().get(obj.fid).fps as u64);
    }
}

impl Sequence for FrameAnim {
    fn update(&mut self, ctx: &mut Context) -> Result {
        match self.state {
            State::Started => {
                self.init(ctx.world);
                ctx.world.objects_mut().reset_screen_shift(&self.obj);
            },
            State::Running(last_time) => {
                if ctx.time - last_time < self.frame_len {
                    return Result::Running(Running::NotLagging);
                }
            }
            State::Done => return Result::Done(Done::AdvanceLater),
        }

        let shift = {
            let mut obj = ctx.world.objects().get(&self.obj).borrow_mut();

            let frame_set = ctx.world.frm_db().get(obj.fid);
            let frames = &frame_set.frame_lists[obj.direction].frames;

            if self.state != State::Started {
                match self.direction {
                    AnimDirection::Forward => {
                        if obj.frame_idx + 1 < frames.len() {
                            obj.frame_idx += 1;
                            Some(frames[obj.frame_idx].shift)
                        } else {
                            None
                        }
                    }
                    AnimDirection::Backward => {
                        if obj.frame_idx > 0 {
                            obj.frame_idx -= 1;
                            Some(-frames[obj.frame_idx + 1].shift)
                        } else {
                            None
                        }
                    }
                }
            } else {
                Some(frames[obj.frame_idx].shift)
            }
        };
        if let Some(shift) = shift {
            ctx.world.objects_mut().add_screen_shift(&self.obj, shift);
        } else {
            self.state = State::Done;
            return Result::Done(Done::AdvanceLater);
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