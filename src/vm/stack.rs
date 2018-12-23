use super::{BadValue, Error, Result};
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
        trace!("stack: pushing {:?}", value);
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
            let r = self.vec.remove(last);
            trace!("stack: popped {:?}", r);
            Ok(r)
        }
    }

    pub fn truncate(&mut self, len: usize) -> Result<()> {
        let old_len = self.vec.len();
        if len <= old_len {
            self.vec.truncate(len);
            trace!("stack: truncated from {} to {}", old_len, len);
            Ok(())
        } else {
            Err(Error::StackUnderflow)
        }
    }

    pub fn get(&self, i: usize) -> Result<&Value> {
        self.vec.get(i).ok_or(Error::BadValue(BadValue::Content))
    }

    pub fn get_mut(&mut self, i: usize) -> Result<&mut Value> {
        self.vec.get_mut(i).ok_or(Error::BadValue(BadValue::Content))
    }
}