use std::cell::Cell;
use std::time::Instant;
use std::rc::Rc;

use game::world::World;
use super::*;

#[derive(Clone, Debug)]
pub struct Cancel(Rc<Cell<bool>>);

impl Cancel {
    fn new() -> Self {
        Cancel(Rc::new(Cell::new(false)))
    }

    pub fn cancel(&self) {
        self.0.set(true)
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.get()
    }
}

pub struct Cancellable<T> {
    sequence: T,
    cancel: Cancel,
}

impl<T: Sequence> Cancellable<T> {
    pub(in super) fn new(seq: T) -> (Self, Cancel) {
        let signal = Cancel::new();
        (Self {
            sequence: seq,
            cancel: signal.clone(),
        }, signal)
    }
}

impl<T: Sequence> Sequence for Cancellable<T> {
    fn update(&mut self, time: Instant, world: &mut World) -> Result {
        if self.cancel.is_cancelled() {
            Result::Done(Done::AdvanceNow)
        } else {
            self.sequence.update(time, world)
        }
    }
}