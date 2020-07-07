use crate::game::object::Handle;
use crate::sequence::*;

pub struct Stand {
    obj: Handle,
    done: bool,
}

impl Stand {
    pub fn new(obj: Handle) -> Self {
        Self {
            obj,
            done: false,
        }
    }
}

impl Sequence for Stand {
    fn update(&mut self, ctx: &mut Update) -> Result {
        if self.done {
            Result::Done
        } else {
            ctx.world.objects_mut().make_standing(self.obj);
            self.done = true;
            Result::Running(Running::NotLagging)
        }
    }
}