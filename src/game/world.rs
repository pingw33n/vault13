use std::rc::Rc;

use crate::asset::frm::FrmDb;
use crate::asset::proto::ProtoDb;
use crate::game::object::{self, Object, Objects};
use crate::graphics::{EPoint, Point};
use crate::graphics::geometry::hex::Direction;
use crate::graphics::geometry::map::MapGrid;
use crate::graphics::lighting::light_grid::LightGrid;
use crate::util::two_dim_array::Array2d;
use crate::graphics::geometry::map::ELEVATION_COUNT;

pub struct World {
    proto_db: Rc<ProtoDb>,
    frm_db: Rc<FrmDb>,
    map_grid: MapGrid,
    sqr_tiles: Array2d<(u16, u16)>,
    objects: Objects,
    light_grid: LightGrid,
    dude_obj: Option<object::Handle>,
}

impl World {
    pub fn new(
            proto_db: Rc<ProtoDb>,
            frm_db: Rc<FrmDb>,
            map_grid: MapGrid,
            sqr_tiles: Array2d<(u16, u16)>,
            objects: Objects) -> Self {
        let light_grid = LightGrid::new(map_grid.hex(), ELEVATION_COUNT);
        let mut r = Self {
            proto_db,
            frm_db,
            map_grid,
            sqr_tiles,
            objects,
            light_grid,
            dude_obj: None,
        };
        r.rebuild_light_grid();

        r
    }

    pub fn proto_db(&self) -> &ProtoDb {
        &self.proto_db
    }

    pub fn frm_db(&self) -> &FrmDb {
        &self.frm_db
    }

    pub fn map_grid(&self) -> &MapGrid {
        &self.map_grid
    }

    pub fn map_grid_mut(&mut self) -> &mut MapGrid {
        &mut self.map_grid
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
}



