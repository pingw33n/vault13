use std::time::{Duration, Instant};

use asset::frm::CritterAnim;
use game::object::Handle;
use game::world::World;
use graphics::ElevatedPoint;
use graphics::geometry::Direction;
use graphics::geometry::hex;
use super::{Result, Sequence};

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
    pub fn new(obj: Handle, anim: CritterAnim, path: Vec<Direction>) -> Box<Sequence> {
        let state = if path.is_empty() {
            State::Done
        } else {
            State::Started
        };
        Box::new(Self {
            obj,
            anim,
            frame_len: Duration::from_millis(1000 / 10),
            path,
            state,
            path_pos: 0,
        })
    }

    fn init_step(&mut self, world: &mut World) {
        let mut obj = world.objects().get(&self.obj).borrow_mut();

        obj.direction = self.path[self.path_pos];
        obj.fid = obj.fid
            .critter()
            .unwrap()
            .with_direction(Some(obj.direction))
            .with_anim(self.anim)
            .into();

        self.frame_len = Duration::from_millis(1000 / world.frm_db().get(obj.fid).fps as u64);
    }
}

impl Sequence for Move {
    fn update(&mut self, time: Instant, world: &mut World) -> Result {
        match self.state {
            State::Started => self.init_step(world),
            State::Running(last_time) => {
                if time - last_time < self.frame_len {
                    return Result::Running;
                }
            }
            State::Done => return Result::Done,
        }

        let new_obj_pos = {
            let (shift, pos) = {
                let mut obj = world.objects().get(&self.obj).borrow_mut();

                let frame_set = world.frm_db().get(obj.fid);
                let frames = &frame_set.frame_lists[obj.direction].frames;

                if self.state != State::Started {
                    obj.frame_idx += 1;
                    if obj.frame_idx >= frames.len() {
                        obj.frame_idx = 0;
                    }
                }

                (frames[obj.frame_idx].shift, obj.pos)
            };
            let shift = world.objects_mut().add_screen_shift(&self.obj, shift);

            let dir = self.path[self.path_pos];
            let next_hex_offset = hex::screen_offset(dir);
            if next_hex_offset.x.abs() > 0
                    && shift.x.abs() >= next_hex_offset.x.abs()
                    || next_hex_offset.y.abs() > 0
                    && shift.y.abs() >= next_hex_offset.y.abs() {
                world.objects_mut().add_screen_shift(&self.obj, -next_hex_offset);
                let pos = pos.unwrap();
                let pos_point = world.map_grid().hex().go(pos.point, dir, 1).unwrap();
                Some(ElevatedPoint::new(pos.elevation, pos_point))
            } else {
                None
            }
        };
        if let Some(pos) = new_obj_pos {
            world.set_object_pos(&self.obj, pos);
            self.path_pos += 1;
            if self.path_pos >= self.path.len() {
                self.state = State::Done;
                return Result::Done;
            }
            self.init_step(world);
        }
        let new_last_time = if let State::Running(last_time) = self.state {
            last_time + self.frame_len
        } else {
            time
        };
        self.state = State::Running(new_last_time);

        if time - new_last_time < self.frame_len {
            Result::Running
        } else {
            Result::Lagging
        }
    }
}