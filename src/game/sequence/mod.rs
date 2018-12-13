pub mod impls;
pub mod then;

use std::time::Instant;

use game::world::World;

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq)]
pub enum Running {
    /// The sequence is not lagging. The caller must not call `update()` again.
    NotLagging,

    /// The sequence is lagging. The caller must repeatedly call `update()` until it returns
    /// `Result::Running(Running::NotLagging)` status or `Result::Done(_)`.
    Lagging,
}

#[derive(Clone, Copy, Debug, Enum, Eq, PartialEq)]
pub enum Done {
    /// If applicable the caller must advance to the next sequence immediately.
    AdvanceNow,

    /// If applicable the caller must defer advancing to the next iteration or tick.
    AdvanceLater,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Result {
    /// Sequence is still running after the `update()` call.
    Running(Running),

    /// Sequence is finished.
    Done(Done),
}

pub trait Sequence {
    fn update(&mut self, time: Instant, world: &mut World) -> Result;

    fn then<T: Sequence>(self, seq: T) -> then::Then<Self, T>
        where Self: Sized
    {
        then::Then::new(self, seq)
    }
}

impl<T: Sequence + ?Sized> Sequence for Box<T> {
    fn update(&mut self, time: Instant, world: &mut World) -> Result {
        (**self).update(time, world)
    }
}

pub struct Sequencer {
    sequences: Vec<Box<Sequence>>,
}

impl Sequencer {
    pub fn new() -> Self {
        Self {
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

    pub fn update(&mut self, time: Instant, world: &mut World) {
        let mut i = 0;
        while i < self.sequences.len() {
            let done = {
                let seq = &mut self.sequences[i];
                update_while_lagging(seq, time, world) == NoLagResult::Done
            };
            if done {
                self.sequences.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NoLagResult {
    Running,
    Done,
}

fn update_while_lagging(mut seq: impl AsMut<Sequence>, time: Instant, world: &mut World) -> NoLagResult {
    loop {
        break match seq.as_mut().update(time, world) {
            Result::Running(Running::Lagging) => continue,
            Result::Running(Running::NotLagging) => NoLagResult::Running,
            Result::Done(_) => NoLagResult::Done,
        };
    }
}