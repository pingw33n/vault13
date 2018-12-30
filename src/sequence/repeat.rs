use super::*;

pub struct Repeat<T, F> {
    seq_producer: F,
    seq: Option<T>,
}

impl<T: Sequence, F: FnMut() -> T> Repeat<T, F> {
    pub fn forever(seq_producer: F) -> Self {
        Self {
            seq_producer,
            seq: None,
        }
    }
}

impl<T: Sequence, F: FnMut() -> T> Sequence for Repeat<T, F> {
    fn update(&mut self, ctx: &mut Context) -> Result {
        if self.seq.is_none() {
            self.seq = Some((self.seq_producer)());
        }
        match self.seq.as_mut().unwrap().update(ctx) {
            r @ Result::Running(_) => r,
            Result::Done(Done::AdvanceLater) => {
                self.seq = None;
                Result::Running(Running::NotLagging)
            },
            Result::Done(Done::AdvanceNow) =>
                panic!("infinite loop in Repeat caused by Done::AdvanceNow result from inner sequence"),
        }
    }
}