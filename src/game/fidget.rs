use log::*;
use std::cmp;
use std::time::{Duration, Instant};

use crate::asset::{CritterAnim, EntityKind, Flag};
use crate::game::sequence::frame_anim::*;
use crate::game::sequence::stand::Stand;
use crate::game::world::World;
use crate::graphics::{EPoint, Point, Rect};
use crate::graphics::geometry::TileGridView;
use crate::sequence::{Sequence, Sequencer};
use crate::util::random::random;

pub struct Fidget {
    next_time: Instant,
}

impl Fidget {
    pub fn new(now: Instant) -> Self {
        Self {
            next_time: now + Self::next_delay(0),
        }
    }

    // dude_fidget()
    pub fn update(&mut self,
        time: Instant,
        world: &mut World,
        sequencer: &mut Sequencer)
    {
        if time < self.next_time {
            return;
        }

        let elevation = world.elevation();

        let hex_rect = world.camera().hex().from_screen_rect(Rect {
            left: world.camera().viewport.left - 320,
            top: world.camera().viewport.top - 190,
            right: world.camera().viewport.width() + 320,
            bottom: world.camera().viewport.height() + 190
        });

        let mut objs = Vec::new();
        for y in hex_rect.top..hex_rect.bottom {
            for x in hex_rect.left..hex_rect.right {
                for &objh in world.objects().at(EPoint::new(elevation, Point::new(x, y))) {
                    let obj = world.objects().get(objh);
                    if obj.flags.contains(Flag::TurnedOff) ||
                        obj.fid.kind() != EntityKind::Critter ||
                        obj.is_critter_dead() ||
                        !world.object_bounds(objh, true).intersects(world.camera().viewport)
                    // FIXME
                    // g_map_header.map_id == MAP_ID_WOODSMAN_ENCOUNTER && obj.pid == Some(Pid::ENCLAVE_PATROL)
                    {
                        continue;
                    }
                    objs.push(objh);
                }
            }
        }

        if !objs.is_empty() {
            let objh = objs[random(0, objs.len() as i32 - 1) as usize];
            let mut obj = world.objects().get_mut(objh);

            if obj.has_running_sequence() {
                debug!("fidget: object {:?} already has a running sequence", objh);
                return;
            }

            // FIXME
            //        if ( obj == g_obj_dude
            //          || ((art_name[0] = 0, art_get_base_name_(OBJ_TYPE_CRITTER, obj->art_fid & 0xFFF, art_name), art_name[0] == 'm')
            //           || art_name[0] == 'M')
            //          && (distance = 2 * stat_level_(g_obj_dude, STAT_PER), obj_dist_(obj, g_obj_dude) <= distance) )
            //        {
            //          play_sfx = 1;
            //        }
            //        if ( play_sfx )
            //        {
            //          sfx_name = gsnd_build_character_sfx_name_(obj, 0, 0);
            //          register_object_play_sfx_((int)obj, (int)sfx_name, 0);
            //        }

            let (seq, cancel) = FrameAnim::new(objh,
                FrameAnimOptions { anim: Some(CritterAnim::Stand), ..Default::default() })
                .cancellable();
            obj.sequence = Some(cancel);
            sequencer.start(seq.then(Stand::new(objh)));

            debug!("fidget: started fidget animation for object {:?}", objh);
        } else {
            debug!("fidget: no suitable objects");
        }

        self.next_time = time + Self::next_delay(objs.len());
    }

    fn next_delay(obj_count: usize) -> Duration {
        let factor = if obj_count == 0 {
            7
        } else {
            cmp::min(cmp::max(20 / obj_count, 1), 7)
        };
        let next_delay = random(0, 3000) + 1000 * factor as i32;
        Duration::from_millis(next_delay as u64)
    }
}