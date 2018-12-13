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
        loop {
            break match self.state {
                State::Seq => {
                    let r = self.seq.update(time, world);
                    match r {
                        Result::Done(d) => {
                            self.state = State::AlwaysSeq;
                            if d == Done::AdvanceNow {
                                continue;
                            }
                            Result::Running(Running::NotLagging)
                        }
                        _ => r,
                    }
                }
                State::AlwaysSeq => self.always_seq.update(time, world),
            }
        }
    }
}