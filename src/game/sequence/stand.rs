use crate::game::object::Handle;
use crate::sequence::*;

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
    fn update(&mut self, ctx: &mut Context) -> Result {
        ctx.world.make_object_standing(self.obj);
        Result::Done(Done::AdvanceLater)
    }
}