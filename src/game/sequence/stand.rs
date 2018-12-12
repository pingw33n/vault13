use game::object::Handle;
use super::*;

pub struct Stand {
    obj: Handle,
}

impl Stand {
    pub fn new(obj: Handle) -> Self {
        Self {
            obj,
        }
    }
}

impl Sequence for Stand {
    fn update(&mut self, _time: Instant, world: &mut World) -> Result {
        world.make_object_standing(&self.obj);
        Result::Done
    }
}