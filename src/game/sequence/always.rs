use std::time::Instant;

use game::sequence::Sequence;
use game::world::World;
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum State {
    Seq,
    AlwaysSeq,
}

pub struct Always {
    state: State,
    seq: Box<Sequence>,
    always_seq: Box<Sequence>,
}

impl Always {
    pub fn new(seq: Box<Sequence>, always_seq: Box<Sequence>) -> Box<Self> {
        Box::new(Self {
            state: State::Seq,
            seq,
            always_seq,
        })
    }
}

impl Sequence for Always {
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