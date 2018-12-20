use std::rc::Rc;

use super::{BadValue, Error, Result, StringMap};
use game::object::Handle;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Int(i32),
    Float(f32),
    String(StringValue),
    Object(Handle),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StringValue {
    Indirect(usize),
    Direct(Rc<String>),
}

impl StringValue {
    pub fn resolve(self, map: &StringMap) -> Result<Rc<String>> {
        Ok(match self {
            StringValue::Indirect(id) =>
                map.get(id).ok_or(Error::BadValue(BadValue::Content))?.clone(),
            StringValue::Direct(s) => s,
        })
    }
}

impl Value {
    pub fn into_int(self) -> Result<i32> {
        if let Value::Int(v) = self {
            Ok(v)
        } else {
            Err(Error::BadValue(BadValue::Type))
        }
    }

    pub fn into_float(self) -> Result<f32> {
        if let Value::Float(v) = self {
            Ok(v)
        } else {
            Err(Error::BadValue(BadValue::Type))
        }
    }

    pub fn into_string_value(self) -> Result<StringValue> {
        if let Value::String(v) = self {
            Ok(v)
        } else {
            Err(Error::BadValue(BadValue::Type))
        }
    }

    pub fn into_string(self, map: &StringMap) -> Result<Rc<String>> {
        self.into_string_value()?.resolve(map)
    }
}