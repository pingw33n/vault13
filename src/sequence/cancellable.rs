use std::cell::Cell;
use std::rc::Rc;

use super::*;

#[derive(Debug)]
pub struct Cancel(Rc<Cell<bool>>);

impl Cancel {
    fn new() -> Self {
        Cancel(Rc::new(Cell::new(false)))
    }

    pub fn cancel(self) {
        self.0.set(true)
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.get()
    }

    fn clone(&self) -> Self {
        Cancel(self.0.clone())
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
    fn update(&mut self, ctx: &mut Context) -> Result {
        if self.cancel.is_cancelled() {
            Result::Done
        } else {
            self.sequence.update(ctx)
        }
    }
}