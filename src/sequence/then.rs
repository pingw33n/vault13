use super::*;

pub struct Then<U, V> {
    first: Option<U>,
    second: V,
}

impl<U: Sequence, V: Sequence> Then<U, V> {
    pub(in super::super) fn new(seq: U, always_seq: V) -> Self {
        Self {
            first: Some(seq),
            second: always_seq,
        }
    }
}

impl<U: Sequence, V: Sequence> Sequence for Then<U, V> {
    fn update(&mut self, ctx: &mut Context) -> Result {
        loop {
            break if self.first.is_some() {
                let r = self.first.as_mut().unwrap().update(ctx);
                match r {
                    Result::Done(d) => {
                        self.first = None;
                        if d == Done::AdvanceNow {
                            continue;
                        }
                        Result::Running(Running::NotLagging)
                    }
                    _ => r,
                }
            } else {
                self.second.update(ctx)
            };
        }
    }
}