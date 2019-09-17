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
        self.set_done();
    }

    pub fn is_done(&self) -> bool {
        self.0.get()
    }

    pub fn is_running(&self) -> bool {
        !self.is_done()
    }

    fn clone(&self) -> Self {
        Cancel(self.0.clone())
    }

    fn set_done(&self) {
        self.0.set(true)
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
    fn update(&mut self, ctx: &mut Update) -> Result {
        if self.cancel.is_done() {
            Result::Done
        } else {
            match self.sequence.update(ctx) {
                r @ Result::Running(_) => r,
                Result::Done => {
                    self.cancel.set_done();
                    Result::Done
                }
            }
        }
    }
}