use super::{Error, Result};
use super::value::Value;

#[derive(Debug)]
pub struct Stack {
    vec: Vec<Value>,
    max_len: usize,
}

impl Stack {
    pub fn new(max_len: usize) -> Self {
        Self {
            vec: Vec::new(),
            max_len,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }

    pub fn len(&self) -> usize {
        self.vec.len()
    }

    pub fn top(&self) -> Option<&Value> {
        self.vec.last()
    }

    pub fn push(&mut self, value: Value) -> Result<()> {
        if self.len() < self.max_len {
            self.vec.push(value);
            Ok(())
        } else {
            Err(Error::StackOverflow)
        }
    }

    pub fn pop(&mut self) -> Result<Value> {
        if self.is_empty() {
            Err(Error::StackUnderflow)
        } else {
            let last = self.len() - 1;
            Ok(self.vec.remove(last))
        }
    }

    pub fn truncate(&mut self, len: usize) -> Result<()> {
        if len <= self.vec.len() {
            self.vec.truncate(len);
            Ok(())
        } else {
            Err(Error::StackUnderflow)
        }
    }
}