pub mod cancellable;
pub mod chain;
pub mod send_event;

use std::time::Instant;

use crate::event::{Sink, Events};
use crate::game::world::World;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Running {
    /// The sequence is not lagging. The caller must not call `update()` again.
    NotLagging,

    /// The sequence is lagging. The caller must repeatedly call `update()` until it returns
    /// `Result::Running(Running::NotLagging)` status or `Result::Done(_)`.
    Lagging,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Result {
    /// Sequence is still running after the `update()` call.
    Running(Running),

    /// Sequence is finished.
    /// If applicable the caller must advance to the next sequence immediately.
    Done,
}

pub struct Update<'a, 'b: 'a> {
    pub time: Instant,
    pub world: &'a mut World,
    pub ui: &'a mut crate::ui::Ui,
    pub sink: &'a mut Sink<'b>,
}

pub trait Sequence {
    fn update(&mut self, ctx: &mut Update) -> Result;

    fn cancellable(self) -> (cancellable::Cancellable<Self>, cancellable::Cancel)
        where Self: Sized
    {
        cancellable::Cancellable::new(self)
    }
}

impl<T: Sequence + ?Sized> Sequence for Box<T> {
    fn update(&mut self, ctx: &mut Update) -> Result {
        (**self).update(ctx)
    }
}

pub struct Sync<'a> {
    pub world: &'a mut World,
    pub ui: &'a mut crate::ui::Ui,
}

pub struct Sequencer {
    last_time: Instant,
    sequences: Vec<Box<dyn Sequence>>,
}

impl Sequencer {
    pub fn new(now: Instant) -> Self {
        Self {
            last_time: now,
            sequences: Vec::new(),
        }
    }

    pub fn is_running(&self) -> bool {
        !self.sequences.is_empty()
    }

    pub fn start(&mut self, sequence: impl 'static + Sequence) {
        self.sequences.push(Box::new(sequence));
    }

    pub fn stop_all(&mut self) {
        self.sequences.clear();
    }

    pub fn update(&mut self, ctx: &mut Update) {
        assert!(self.last_time <= ctx.time);
        self.last_time = ctx.time;
        let mut i = 0;
        while i < self.sequences.len() {
            let done = {
                let seq = &mut self.sequences[i];
                update_while_lagging(seq, ctx) == NoLagResult::Done
            };
            if done {
                self.sequences.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    /// Executes a no-advance update so the effect of cancellation can be seen immediately.
    pub fn sync(&mut self, ctx: &mut Sync) {
        let mut events = Events::new();
        self.update(&mut Update {
            time: self.last_time,
            world: ctx.world,
            ui: ctx.ui,
            sink: &mut events.sink(),
        });
        assert!(events.is_empty());
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NoLagResult {
    Running,
    Done,
}

fn update_while_lagging(mut seq: impl AsMut<dyn Sequence>, ctx: &mut Update) -> NoLagResult {
    loop {
        break match seq.as_mut().update(ctx) {
            Result::Running(Running::Lagging) => continue,
            Result::Running(Running::NotLagging) => NoLagResult::Running,
            Result::Done => NoLagResult::Done,
        };
    }
}