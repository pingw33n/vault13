use std::time::{Duration, Instant};

use crate::asset::CritterAnim;
use crate::game::object::Handle;
use crate::game::world::World;
use crate::graphics::EPoint;
use crate::graphics::geometry::hex::{self, Direction};
use crate::sequence::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum State {
    Started,
    Running(Instant),
    Done,
}

pub struct Move {
    obj: Handle,
    anim: CritterAnim,
    frame_len: Duration,
    path: Vec<Direction>,
    state: State,
    path_pos: usize,
}

impl Move {
    pub fn new(obj: Handle, anim: CritterAnim, path: Vec<Direction>) -> Self {
        let state = if path.is_empty() {
            State::Done
        } else {
            State::Started
        };
        Self {
            obj,
            anim,
            frame_len: Duration::from_millis(1000 / 10),
            path,
            state,
            path_pos: 0,
        }
    }

    fn init_step(&mut self, world: &mut World) {
        let mut obj = world.objects().get(self.obj).borrow_mut();

        obj.direction = self.path[self.path_pos];
        obj.fid = obj.fid
            .critter()
            .unwrap()
            .with_direction(Some(obj.direction))
            .with_anim(self.anim)
            .into();

        if self.state == State::Started {
            obj.frame_idx = 0;
        }

        self.frame_len = Duration::from_millis(1000 / world.frm_db().get(obj.fid).fps as u64);
    }
}

impl Sequence for Move {
    // object_move()
    fn update(&mut self, ctx: &mut Context) -> Result {
        match self.state {
            State::Started => {
                self.init_step(ctx.world);
                ctx.world.objects_mut().reset_screen_shift(self.obj);
            },
            State::Running(last_time) => {
                if ctx.time - last_time < self.frame_len {
                    return Result::Running(Running::NotLagging);
                }
            }
            State::Done => return Result::Done,
        }

        let new_obj_pos_and_shift = {
            let (shift, pos) = {
                let mut obj = ctx.world.objects().get(self.obj).borrow_mut();

                let frame_set = ctx.world.frm_db().get(obj.fid);
                let frames = &frame_set.frame_lists[obj.direction].frames;

                if self.state != State::Started {
                    obj.frame_idx += 1;
                    if obj.frame_idx >= frames.len() {
                        obj.frame_idx = 0;
                    }
                }

                (frames[obj.frame_idx].shift, obj.pos)
            };
            let shift = ctx.world.objects_mut().add_screen_shift(self.obj, shift);

            let dir = self.path[self.path_pos];
            let next_hex_offset = hex::screen_offset(dir);
            if next_hex_offset.x != 0
                    && shift.x.abs() >= next_hex_offset.x.abs()
                    || next_hex_offset.y != 0
                    && shift.y.abs() >= next_hex_offset.y.abs() {
                let shift = {
                    let obj = ctx.world.objects().get(self.obj).borrow();
                    obj.screen_shift - next_hex_offset
                };
                let pos = pos.unwrap();
                let pos_point = ctx.world.hex_grid().go(pos.point, dir, 1).unwrap();
                Some((EPoint::new(pos.elevation, pos_point), shift))
            } else {
                None
            }
        };
        if let Some((pos, shift)) = new_obj_pos_and_shift {
            ctx.world.set_object_pos(self.obj, pos);

            self.path_pos += 1;
            if self.path_pos >= self.path.len() {
                self.state = State::Done;
                return Result::Done;
            }
            ctx.world.objects_mut().add_screen_shift(self.obj, shift);
            self.init_step(ctx.world);
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