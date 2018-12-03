use asset::frm::FrmDb;
use asset::proto::ProtoDb;
use game::object::{self, Object, Objects};
use graphics::ElevatedPoint;
use graphics::geometry::map::MapGrid;
use graphics::lighting::light_grid::LightGrid;
use util::two_dim_array::Array2d;
use graphics::geometry::map::ELEVATION_COUNT;

use std::rc::Rc;

pub struct World {
    proto_db: Rc<ProtoDb>,
    frm_db: Rc<FrmDb>,
    map_grid: MapGrid,
    sqr_tiles: Array2d<(u16, u16)>,
    objects: Objects,
    light_grid: LightGrid,
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
        };
        r.rebuild_light_grid();

        r
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

    pub fn light_grid(&self) -> &LightGrid {
        &self.light_grid
    }

    pub fn insert_object(&mut self, object: Object) -> object::Handle {
        let h = self.objects.insert(object);

        Self::update_light_grid(&self.objects, &mut self.light_grid, &h, 1);

        h
    }

    pub fn set_object_pos(&mut self, h: &object::Handle, pos: impl Into<ElevatedPoint>) {
        Self::update_light_grid(&self.objects, &mut self.light_grid, h, -1);

        self.objects.set_pos(h, pos);

        Self::update_light_grid(&self.objects, &mut self.light_grid, h, 1);
    }

    pub fn rebuild_light_grid(&mut self) {
        self.light_grid.clear();
        for ref h in self.objects.iter() {
            Self::update_light_grid(&self.objects, &mut self.light_grid, h, 1);
        }
    }

    fn update_light_grid(objects: &Objects, light_grid: &mut LightGrid, h: &object::Handle,
            factor: i32) {
        let obj = objects.get(&h).borrow();
        if let Some(pos) = obj.pos {
            light_grid.update(pos,
                obj.light_emitter.radius,
                factor * obj.light_emitter.intensity as i32,
                |lt| objects.light_test(lt));
        }
    }
}



