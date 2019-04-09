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
        const MAX_ITERS: usize = 10;
        for _ in 0..MAX_ITERS {
            if self.seq.is_none() {
                self.seq = Some((self.seq_producer)());
            }
            match self.seq.as_mut().unwrap().update(ctx) {
                r @ Result::Running(_) => return r,
                Result::Done => self.seq = None,
            }
        }
        panic!("inner sequence didn't start after {} iterations", MAX_ITERS);
    }
}