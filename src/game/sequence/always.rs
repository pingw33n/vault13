use std::time::Instant;

use game::sequence::Sequence;
use game::world::World;
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum State {
    Seq,
    AlwaysSeq,
}

pub struct Always<U, V> {
    state: State,
    seq: U,
    always_seq: V,
}

impl<U: Sequence, V: Sequence> Always<U, V> {
    pub fn new(seq: U, always_seq: V) -> Self {
        Self {
            state: State::Seq,
            seq,
            always_seq,
        }
    }
}

impl<U: Sequence, V: Sequence> Sequence for Always<U, V> {
    fn update(&mut self, time: Instant, world: &mut World) -> Result {
        match self.state {
            State::Seq => {
                let r = self.seq.update(time, world);
                if r == Result::Done {
                    self.state = State::AlwaysSeq;
                    Result::Running
                } else {
                    r
                }
            }
            State::AlwaysSeq => self.always_seq.update(time, world),
        }
    }
}