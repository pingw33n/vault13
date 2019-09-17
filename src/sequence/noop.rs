use super::*;

pub struct Noop(());

impl Noop {
    pub fn new() -> Self {
        Noop(())
    }
}

impl Sequence for Noop {
    fn update(&mut self, _ctx: &mut Update) -> Result {
        Result::Done
    }
}