use enum_map_derive::Enum;

pub mod cancellable;
pub mod chain;
pub mod event;
pub mod noop;
pub mod repeat;
pub mod sleep;
pub mod then;

use std::time::Instant;

use crate::game::world::World;

pub use event::Event;

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq)]
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

pub struct Update<'a> {
    pub time: Instant,
    pub world: &'a mut World,
    pub out: &'a mut Vec<Event>,
}

pub trait Sequence {
    fn update(&mut self, ctx: &mut Update) -> Result;

    fn then<T: Sequence>(self, seq: T) -> then::Then<Self, T>
        where Self: Sized
    {
        then::Then::new(self, seq)
    }

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

pub struct Cleanup<'a> {
    pub world: &'a mut World,
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

    /// Executes a no-advance update so the cancelled sequenced can get cleaned up.
    pub fn cleanup(&mut self, ctx: &mut Cleanup) {
        let out = &mut Vec::new();
        self.update(&mut Update {
            time: self.last_time,
            world: ctx.world,
            out,
        });
        assert!(out.is_empty());
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NoLagResult {
    Running,
    Done,
}

fn update_while_lagging(mut seq: impl AsMut<Sequence>, ctx: &mut Update) -> NoLagResult {
    loop {
        break match seq.as_mut().update(ctx) {
            Result::Running(Running::Lagging) => continue,
            Result::Running(Running::NotLagging) => NoLagResult::Running,
            Result::Done => NoLagResult::Done,
        };
    }
}