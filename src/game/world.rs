use std::cmp;
use std::rc::Rc;

use crate::asset::EntityKind;
use crate::asset::frame::{FrameId, FrameDb};
use crate::asset::map::ELEVATION_COUNT;
use crate::asset::proto::ProtoDb;
use crate::game::GameTime;
use crate::game::object::{self, DamageFlag, Egg, Object, Objects, SubObject};
use crate::graphics::{EPoint, Point, Rect};
use crate::graphics::geometry::camera::Camera;
use crate::graphics::geometry::hex::{self, Direction};
use crate::graphics::lighting::light_grid::LightGrid;
use crate::graphics::map::*;
use crate::graphics::render::Canvas;
use crate::util::array2d::Array2d;

pub struct World {
    proto_db: Rc<ProtoDb>,
    frm_db: Rc<FrameDb>,
    hex_grid: hex::TileGrid,
    camera: Camera,
    sqr_tiles: Vec<Option<Array2d<(u16, u16)>>>,
    objects: Objects,
    light_grid: LightGrid,
    dude_obj: Option<object::Handle>,
    pub game_time: GameTime,
    pub ambient_light: u32,
}

impl World {
    pub fn new(
            proto_db: Rc<ProtoDb>,
            frm_db: Rc<FrameDb>,
            hex_grid: hex::TileGrid,
            viewport: Rect,
            sqr_tiles: Vec<Option<Array2d<(u16, u16)>>>,
            objects: Objects) -> Self {
        assert_eq!(sqr_tiles.len(), ELEVATION_COUNT as usize);
        let light_grid = LightGrid::new(
            hex_grid.width(),
            hex_grid.height(),
            ELEVATION_COUNT);
        let mut r = Self {
            proto_db,
            frm_db,
            hex_grid,
            camera: Camera {
                origin: Point::new(0, 0),
                viewport,
            },
            sqr_tiles,
            objects,
            light_grid,
            dude_obj: None,
            game_time: GameTime::from_decis(0),
            ambient_light: 0x10000,
        };
        r.rebuild_light_grid();

        r
    }

    pub fn proto_db(&self) -> &ProtoDb {
        &self.proto_db
    }

    pub fn frm_db(&self) -> &FrameDb {
        &self.frm_db
    }

    pub fn hex_grid(&self) -> &hex::TileGrid {
        &self.hex_grid
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    pub fn objects(&self) -> &Objects {
        &self.objects
    }

    pub fn objects_mut(&mut self) -> &mut Objects {
        &mut self.objects
    }

    pub fn light_grid(&self) -> &LightGrid {
        &self.light_grid
    }

    pub fn insert_object(&mut self, object: Object) -> object::Handle {
        let h = self.objects.insert(object);

        Self::update_light_grid(&self.objects, &mut self.light_grid, h, 1);

        h
    }

    pub fn dude_obj(&self) -> Option<object::Handle> {
        self.dude_obj
    }

    pub fn set_dude_obj(&mut self, obj: object::Handle) {
        assert!(self.objects.contains(obj));
        assert!(self.dude_obj.is_none());
        self.dude_obj = Some(obj);
    }

    pub fn elevation(&self) -> u32 {
        self.objects.get(self.dude_obj.expect("no dude_obj")).borrow()
            .pos.expect("dude_obj has no pos")
            .elevation
    }

    pub fn has_elevation(&self, elevation: u32) -> bool {
        self.sqr_tiles[elevation as usize].is_some()
    }

    pub fn set_object_pos(&mut self, h: object::Handle, pos: impl Into<EPoint>) {
        Self::update_light_grid(&self.objects, &mut self.light_grid, h, -1);

        self.objects.set_pos(h, pos);

        Self::update_light_grid(&self.objects, &mut self.light_grid, h, 1);
    }

    pub fn make_object_standing(&mut self, h: object::Handle) {
        self.objects.make_standing(h, &self.frm_db);
    }

    pub fn path_for_object(&self, obj: object::Handle, to: impl Into<Point>, smooth: bool)
            -> Option<Vec<Direction>> {
        self.objects.path_for_object(obj, to, smooth, &self.proto_db)
    }

    pub fn rebuild_light_grid(&mut self) {
        self.light_grid.clear();
        for h in self.objects.iter() {
            Self::update_light_grid(&self.objects, &mut self.light_grid, h, 1);
        }
    }

    pub fn object_bounds(&self, obj: object::Handle) -> Rect {
        self.objects.bounds(obj, &self.camera.hex())
    }

    pub fn object_hit_test(&self, p: impl Into<Point>) -> Vec<(object::Handle, object::Hit)> {
        self.objects.hit_test(p.into().elevated(self.elevation()), &self.camera.viewport,
            &self.camera.hex(), self.egg())
    }

    // object_under_mouse()
    pub fn pick_object(&self, pos: impl Into<Point>, include_dude: bool) -> Option<object::Handle> {
        let filter_dude = |oh: &&(object::Handle, object::Hit)| -> bool {
            include_dude || Some(oh.0) != self.dude_obj
        };
        let hits = self.object_hit_test(pos);
        let r = hits
            .iter()
            .filter(filter_dude)
            .filter(|(_, h)| !h.translucent && !h.with_egg)
            .filter(|&&(o, _)| {
                let obj = self.objects.get(o).borrow();
                if let Some(SubObject::Critter(critter)) = &obj.sub {
                    !critter.combat.damage_flags.intersects(DamageFlag::Dead | DamageFlag::KnockedOut)
                } else {
                    true
                }
            })
            .map(|&(o, _)| o)
            .next();
        if r.is_none() {
            hits.iter()
                .filter(filter_dude)
                .map(|&(o, _)| o)
                .next()
        } else {
            r
        }
    }

    pub fn render(&self, canvas: &mut Canvas, draw_roof: bool) {
        let elevation = self.elevation();
        render_floor(canvas, &self.camera.sqr(), &self.camera.viewport,
            |p| {
                let fid = FrameId::new_generic(EntityKind::SqrTile,
                    self.sqr_tiles[elevation as usize].as_ref().unwrap().get(p.x as usize, p.y as usize).unwrap().0).unwrap();
                Some(self.frm_db.get(fid).frame_lists[Direction::NE].frames[0].texture.clone())
            },
            |point| {
                let l = self.light_grid().get_clipped(EPoint { elevation, point });
                cmp::max(l, self.ambient_light)
            }
        );

        self.objects().render(canvas, elevation, &self.camera.viewport, &self.camera.hex(),
            self.egg().as_ref(),
            |pos| if let Some(pos) = pos {
                cmp::max(self.light_grid().get_clipped(pos), self.ambient_light)
            } else {
                self.ambient_light
            });

        if draw_roof {
            render_roof(canvas, &self.camera.sqr(), &self.camera.viewport,
                |p| Some(self.frm_db.get(FrameId::new_generic(EntityKind::SqrTile,
                    self.sqr_tiles[elevation as usize].as_ref().unwrap().get(p.x as usize, p.y as usize).unwrap().1).unwrap())
                        .first().texture.clone()));
        }

        self.objects().render_outlines(canvas, elevation, &self.camera.viewport, &self.camera.hex());

        // TODO render floating text objects.
    }

    fn update_light_grid(objects: &Objects, light_grid: &mut LightGrid, h: object::Handle,
            factor: i32) {
        let obj = objects.get(h).borrow();
        if let Some(pos) = obj.pos {
            light_grid.update(pos,
                obj.light_emitter.radius,
                factor * obj.light_emitter.intensity as i32,
                |lt| objects.light_test(lt));
        }
    }

    fn egg(&self) -> Option<Egg> {
        if let Some(dude_obj) = self.dude_obj {
            Some(Egg {
                pos: self.objects().get(dude_obj).borrow().pos.unwrap().point,
                fid: FrameId::EGG,
            })
        } else {
            None
        }
    }
}



