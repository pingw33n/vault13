pub mod always;
pub mod move_seq;
pub mod stand;

use std::time::Instant;

use game::world::World;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Result {
    Running,
    Lagging,
    Done,
}

pub trait Sequence {
    fn update(&mut self, time: Instant, world: &mut World) -> Result;
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
        match seq.as_mut().update(time, world) {
            Result::Running => break NoLagResult::Running,
            Result::Lagging => {},
            Result::Done => break NoLagResult::Done,
        }
    }
}