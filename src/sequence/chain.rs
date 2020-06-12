use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use super::*;

#[derive(Clone, Copy, Debug)]
enum State {
    Cancellable,
    Finalizing,
    Done,
}

struct Inner {
    on_done: Option<Box<dyn FnOnce()>>,
    new_cancellable: Vec<Box<dyn Sequence>>,
    new_finalizing: Vec<Box<dyn Sequence>>,
    state: State,
}

impl Inner {
    fn new() -> Self {
        Self {
            on_done: None,
            new_cancellable: Vec::new(),
            new_finalizing: Vec::new(),
            state: State::Cancellable,
        }
    }

    fn flush(&mut self,
        cancellable: &mut VecDeque<Box<dyn Sequence>>,
        finalizing: &mut VecDeque<Box<dyn Sequence>>,
    ) {
        cancellable.extend(self.new_cancellable.drain(..));
        finalizing.extend(self.new_finalizing.drain(..));
    }
}

#[derive(Clone)]
pub struct Control(Rc<RefCell<Inner>>);

impl Control {
    /// Appends a new sequence to the cancellable sub-chain.
    /// Calling `cancel()` will cancel any running or pending cancellable sequence.
    /// Panics if the cancellable sub-chain has already finished running.
    pub fn cancellable(&self, seq: impl 'static + Sequence) -> &Self {
        let mut inner = self.0.borrow_mut();
        match inner.state {
            State::Cancellable => {},
            State::Finalizing | State::Done => panic!(
                "can't push cancellable sequence because the cancellable sub-chain has already finished running"),
        }
        inner.new_cancellable.push(Box::new(seq));
        self
    }

    /// Appends a new sequence to the finalizing sub-chain.
    /// Finalizing sequences run after all cancellable sequences finished.
    /// Finalizing sequences can't be cancelled.
    /// Panics if the chain has already finished running.
    pub fn finalizing(&self, seq: impl 'static + Sequence) -> &Self {
        let mut inner = self.0.borrow_mut();
        match inner.state {
            State::Cancellable | State::Finalizing => {}
            State::Done => panic!(
                "can't push finalizing sequence because the chain has already finished running"),
        }
        inner.new_finalizing.push(Box::new(seq));
        self
    }

    /// Cancels any running or pending cancellable sequence.
    /// Idempotent, has no effect if already cancelled or the chain is finished running.
    pub fn cancel(&self) -> &Self {
        let mut inner = self.0.borrow_mut();
        match inner.state {
            State::Cancellable => inner.state = State::Finalizing,
            State::Finalizing | State::Done => {}
        }
        self
    }

    /// Sets a callback to be called when the chain finishes.
    pub fn on_done(&self, f: impl 'static + FnOnce()) -> &Self {
        let mut inner = self.0.borrow_mut();
        match inner.state {
            State::Cancellable | State::Finalizing => {},
            State::Done => panic!("already done"),
        }
        assert!(inner.on_done.replace(Box::new(f)).is_none());
        self
    }

    fn new() -> Self {
        Control(Rc::new(RefCell::new(Inner::new())))
    }

    fn done(&self) {
        let on_done = {
            let mut inner = self.0.borrow_mut();
            inner.state = State::Done;
            inner.on_done.take()
        };
        if let Some(on_done) = on_done {
            on_done();
        }
    }
}

/// A cancellable chain of sequences. The sequences are divided into two groups: cancellable and
/// finalizing. Cancellable sequence are run first and can be cancelled. Finalizing sequences
/// run after cancellable sequences finished (either normally or by cancelling) and can't be
/// cancelled.
pub struct Chain {
    cancellable: VecDeque<Box<dyn Sequence>>,
    finalizing: VecDeque<Box<dyn Sequence>>,
    control: Control,
}

impl Chain {
    pub fn new() -> Self {
        Self {
            cancellable: VecDeque::new(),
            finalizing: VecDeque::new(),
            control: Control::new(),
        }
    }

    pub fn control(&self) -> &Control {
        &self.control
    }
}

impl Sequence for Chain {
    fn update(&mut self, ctx: &mut Update) -> Result {
        self.control.0.borrow_mut().flush(&mut self.cancellable, &mut self.finalizing);
        loop {
            let state = self.control.0.borrow().state;
            match state {
                State::Cancellable => {
                    let r = match self.cancellable.front_mut().map(|seq| seq.update(ctx)) {
                        Some(r @ Result::Running(_)) => r,
                        Some(Result::Done) => {
                            self.cancellable.pop_front().unwrap();
                            if self.cancellable.is_empty() {
                                Result::Done
                            } else {
                                continue;
                            }
                        },
                        None => Result::Done,
                    };
                    match r {
                        Result::Done => {
                            self.control.0.borrow_mut().state = State::Finalizing;
                        }
                        Result::Running(_) => break r,
                    }
                }
                State::Finalizing => {
                    let r = match self.finalizing.front_mut().map(|seq| seq.update(ctx)) {
                        Some(r @ Result::Running(_)) => r,
                        Some(Result::Done) => {
                            self.finalizing.pop_front().unwrap();
                            if self.finalizing.is_empty() {
                                Result::Done
                            } else {
                                continue;
                            }
                        }
                        None => Result::Done,
                    };
                    match r {
                        Result::Done => self.control.done(),
                        Result::Running(_) => break r,
                    }
                }
                State::Done => break Result::Done,
            }
        }
    }
}