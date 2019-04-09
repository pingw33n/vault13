use std::cell::RefCell;
use std::rc::Rc;

use super::*;

struct Inner {
    new: Vec<Box<Sequence>>,
    clear: bool,
}

impl Inner {
    fn new() -> Self {
        Self {
            new: Vec::new(),
            clear: false,
        }
    }

    fn push(&mut self, seq: impl 'static + Sequence) {
        self.new.push(Box::new(seq));
    }

    fn clear(&mut self) {
        self.new.clear();
        self.clear = true;
    }

    fn apply(&mut self, seqs: &mut Vec<Box<Sequence>>) {
        if self.clear {
            self.clear = false;
            seqs.clear();
        }
        seqs.extend(self.new.drain(..));
    }
}

#[derive(Clone)]
pub struct Control(Rc<RefCell<Inner>>);

impl Control {
    pub fn push(&self, seq: impl 'static + Sequence) {
        self.0.borrow_mut().push(seq);
    }

    pub fn clear(&self) {
        self.0.borrow_mut().clear();
    }

    fn new() -> Self {
        Control(Rc::new(RefCell::new(Inner::new())))
    }

    fn apply(&self, seqs: &mut Vec<Box<Sequence>>) {
        self.0.borrow_mut().apply(seqs);
    }

    fn is_unique(&self) -> bool {
        Rc::strong_count(&self.0) == 1
    }
}

pub struct Chain {
    sequences: Vec<Box<Sequence>>,
    control: Control,
    keep_running: bool,
}

impl Chain {
    pub fn oneshot() -> (Self, Control) {
        Self::new(false)
    }

    pub fn endless() -> (Self, Control) {
        Self::new(true)
    }

    pub fn push(&mut self, seq: impl 'static + Sequence) {
        self.sequences.push(Box::new(seq));
    }

    fn new(keep_running: bool) -> (Self, Control) {
        let control = Control::new();
        (Self {
            sequences: Vec::new(),
            control: control.clone(),
            keep_running,
        }, control)
    }
}

impl Sequence for Chain {
    fn update(&mut self, ctx: &mut Context) -> Result {
        self.control.apply(&mut self.sequences);
        loop {
            let r = match self.sequences.first_mut().map(|seq| seq.update(ctx)) {
                Some(r @ Result::Running(_)) => r,
                Some(Result::Done) => {
                    self.sequences.remove(0);
                    if self.sequences.is_empty() {
                        if self.keep_running && self.control.is_unique() {
                            panic!("all controls are gone and no more sequences can be added");
                        }
                        Result::Done
                    } else {
                        continue;
                    }
                },
                None => Result::Done,
            };
            break match r {
                Result::Done if self.keep_running => Result::Running(Running::NotLagging),
                Result::Done | Result::Running(_) => r
            }
        }
    }
}