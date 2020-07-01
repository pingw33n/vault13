use std::time::{Duration, Instant};

use crate::asset::CritterAnim;
use crate::game::object::{Handle, PathTo};
use crate::game::world::World;
use crate::graphics::{EPoint, Point};
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
    to: PathTo,
    to_point: Option<Point>,
    anim: CritterAnim,
    frame_len: Duration,
    path: Vec<Direction>,
    state: State,
    path_pos: usize,
}

impl Move {
    pub fn new(obj: Handle, to: PathTo, anim: CritterAnim) -> Self {
        Self {
            obj,
            to,
            to_point: None,
            anim,
            frame_len: Duration::from_millis(1000 / 10),
            path: Vec::new(),
            state: State::Started,
            path_pos: 0,
        }
    }

    fn init_step(&mut self, world: &mut World) {
        let mut obj = world.objects().get_mut(self.obj);

        // Path can be empty in Started state.
        if !self.path.is_empty() {
            obj.direction = self.path[self.path_pos];
        }
        obj.fid = obj.fid
            .critter()
            .unwrap()
            .with_anim(self.anim)
            .into();

        if self.state == State::Started {
            obj.frame_idx = 0;
        }

        self.frame_len = Duration::from_millis(1000 / world.frm_db().get(obj.fid).unwrap().fps as u64);
    }

    fn rebuild_path(&mut self, world: &mut World) {
        // TODO non-smooth
        self.path = world.objects().path(self.obj, self.to, true).unwrap_or(Vec::new());
    }

    fn to_point(&self, world: &World) -> Point {
        match self.to {
            PathTo::Object(h) => world.objects().get(h).pos.unwrap().point,
            PathTo::Point { point, .. } => point,
        }
    }

    fn done(&mut self, ctx: &mut Update) {
        let mut obj = ctx.world.objects().get_mut(self.obj);
        let pos = obj.pos.unwrap().point;
        if pos != self.to_point.unwrap() {
            // Make object look into target's direction.
            obj.direction = hex::direction(pos, self.to_point.unwrap());
        }
        self.state = State::Done;
    }
}

impl Sequence for Move {
    // object_move()
    fn update(&mut self, ctx: &mut Update) -> Result {
        match self.state {
            State::Started => {
                self.rebuild_path(ctx.world);
                // TODO Do we need to rebuild path if the object moves?
                self.to_point = Some(self.to_point(ctx.world));

                self.init_step(ctx.world);
                ctx.world.objects_mut().reset_screen_shift(self.obj);

                if self.path.is_empty() {
                    self.done(ctx);
                    return Result::Done;
                }
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
                let mut obj = ctx.world.objects().get_mut(self.obj);

                let frame_set = ctx.world.frm_db().get(obj.fid).unwrap();
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
            let next_offset = hex::screen_offset(dir);
            if next_offset.x > 0 && shift.x >= next_offset.x ||
                next_offset.x < 0 && shift.x <= next_offset.x ||
                next_offset.y > 0 && shift.y >= next_offset.y ||
                next_offset.y < 0 && shift.y <= next_offset.y
            {
                let shift = {
                    let obj = ctx.world.objects().get(self.obj);
                    obj.screen_shift - next_offset
                };
                let pos = pos.unwrap();
                let pos_point = ctx.world.hex_grid().go(pos.point, dir, 1).unwrap();
                Some((EPoint::new(pos.elevation, pos_point), shift))
            } else {
                None
            }
        };
        if let Some((pos, shift)) = new_obj_pos_and_shift {
            let old_pos = ctx.world.objects().get(self.obj).pos.unwrap();
            ctx.world.set_object_pos(self.obj, Some(pos));

            ctx.out.push(Event::ObjectMoved {
                obj: self.obj,
                old_pos,
                new_pos: pos,
            });

            // TODO check for blocker and rebuild path
            // TODO use door

            self.path_pos += 1;
            if self.path_pos >= self.path.len() {
                self.done(ctx);
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