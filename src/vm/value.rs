use std::cmp::Ordering;
use std::rc::Rc;

use super::{BadValue, Error, Result, StringMap};
use game::object::Handle;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(i32),
    Float(f32),
    String(StringValue),
    Object(Handle),
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

    pub fn into_string(self, strings: &StringMap) -> Result<Rc<String>> {
        self.into_string_value()?.resolve(strings)
    }

    pub fn into_object(self) -> Result<Handle> {
        if let Value::Object(v) = self {
            Ok(v)
        } else {
            Err(Error::BadValue(BadValue::Type))
        }
    }

    pub fn coerce_into_object(self) -> Result<Option<Handle>> {
        if self == Value::Int(0) || self == Value::Float(0.0) {
            Ok(None)
        } else {
            self.into_object().map(|h| Some(h))
        }
    }

    pub fn resolved(self, strings: &StringMap) -> Result<Value> {
        Ok(match self {
            Value::String(v) => Value::String(v.resolved(strings)?),
            v @ _ => v,
        })
    }

    pub fn neg(&self) -> Result<Value> {
        Ok(match self {
            Value::Int(v)      => Value::Int(-*v),
            Value::Float(v)    => Value::Float(-*v),
            Value::String(_)    => return Err(Error::BadValue(BadValue::Type)),
            Value::Object(_)    => return Err(Error::BadValue(BadValue::Type)),
        })
    }

    pub fn not(&self) -> Result<Value> {
        Ok(Value::Int(match self {
            Value::Int(v)      => *v == 0,
            Value::Float(v)    => *v == 0.0,
            Value::String(_)    => false,
            Value::Object(_)    => false,
        } as i32))
    }

    pub fn partial_cmp(&self, other: &Value, strings: &StringMap) -> Result<Option<Ordering>> {
        Fold2 {
            left: self.clone(),
            right: other.clone(),
            int_int         : |left, right| Ok(left.partial_cmp(&right)),
            int_float       : |left, right| Ok((left as f32).partial_cmp(&right)),
            int_string      : |left, right| Ok(left.to_string().partial_cmp(&right)),
            float_int       : |left, right| Ok(left.partial_cmp(&(right as f32))),
            float_float     : |left, right| Ok(left.partial_cmp(&right)),
            float_string    : |left, right| Ok(left.to_string().partial_cmp(&right)),
            string_int      : |left, right| Ok((*left).partial_cmp(&right.to_string())),
            string_float    : |left, right| Ok((*left).partial_cmp(&right.to_string())),
            string_string   : |left, right| Ok(left.partial_cmp(&right)),
        }.apply(strings)
    }

    pub fn test(&self) -> bool {
        match self {
            Value::Int(v) => *v != 0,
            Value::Float(v) => *v != 0.0,
            Value::String(_) => true,
            Value::Object(_) => true,
        }
    }

    pub fn add(self, other: Value, strings: &StringMap) -> Result<Value> {
        use std::fmt;
        fn concat(s1: impl fmt::Display, s2: impl fmt::Display) -> Result<Value> {
            Ok(Value::String(StringValue::Direct(Rc::new(format!("{}{}", s1, s2)))))
        }

        Fold2 {
            left: self,
            right: other,
            int_int         : |left, right| Ok(Value::Int(left + right)),
            int_float       : |left, right| Ok(Value::Float((left as f32) + right)),
            int_string      : |left, right| concat(left.to_string(), right),
            float_int       : |left, right| Ok(Value::Float(left + (right as f32))),
            float_float     : |left, right| Ok(Value::Float(left  + right)),
            float_string    : |left, right| concat(left.to_string(), right),
            string_int      : |left, right| concat(left, right.to_string()),
            string_float    : |left, right| concat(left, right.to_string()),
            string_string   : |left, right| concat(left, right),
        }.apply(strings)
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::Int(v)
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Int(v as i32)
    }
}

impl From<f32> for Value {
    fn from(v: f32) -> Self {
        Value::Float(v)
    }
}

impl From<Rc<String>> for Value {
    fn from(v: Rc<String>) -> Self {
        Value::String(StringValue::Direct(v))
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::String(StringValue::Direct(Rc::new(v)))
    }
}

impl<'a> From<&'a str> for Value {
    fn from(v: &'a str) -> Self {
        Self::from(v.to_string())
    }
}

impl From<Handle> for Value {
    fn from(v: Handle) -> Self {
        Value::Object(v)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StringValue {
    Indirect(usize),
    Direct(Rc<String>),
}

impl StringValue {
    pub fn resolve(self, strings: &StringMap) -> Result<Rc<String>> {
        Ok(self.resolved(strings)?.into_direct().unwrap())

    }
    pub fn resolved(self, strings: &StringMap) -> Result<StringValue> {
        Ok(match self {
            StringValue::Indirect(id) => StringValue::Direct(
                strings.get(id).ok_or(Error::BadValue(BadValue::Content))?.clone()),
            StringValue::Direct(s) => StringValue::Direct(s),
        })
    }

    pub fn into_direct(self) -> Option<Rc<String>> {
        if let StringValue::Direct(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

pub struct Fold2<
    T,
    IntInt      : FnOnce(i32, i32)                  -> Result<T>,
    IntFloat    : FnOnce(i32, f32)                  -> Result<T>,
    IntString   : FnOnce(i32, Rc<String>)           -> Result<T>,
    FloatInt    : FnOnce(f32, i32)                  -> Result<T>,
    FloatFloat  : FnOnce(f32, f32)                  -> Result<T>,
    FloatString : FnOnce(f32, Rc<String>)           -> Result<T>,
    StringInt   : FnOnce(Rc<String>, i32)           -> Result<T>,
    StringFloat : FnOnce(Rc<String>, f32)           -> Result<T>,
    StringString: FnOnce(Rc<String>, Rc<String>)    -> Result<T>,
> {
    pub left            : Value,
    pub right           : Value,
    pub int_int         : IntInt,
    pub int_float       : IntFloat,
    pub int_string      : IntString,
    pub float_int       : FloatInt,
    pub float_float     : FloatFloat,
    pub float_string    : FloatString,
    pub string_int      : StringInt,
    pub string_float    : StringFloat,
    pub string_string   : StringString,
}

impl<
    T,
    IntInt      : FnOnce(i32, i32)                  -> Result<T>,
    IntFloat    : FnOnce(i32, f32)                  -> Result<T>,
    IntString   : FnOnce(i32, Rc<String>)           -> Result<T>,
    FloatInt    : FnOnce(f32, i32)                  -> Result<T>,
    FloatFloat  : FnOnce(f32, f32)                  -> Result<T>,
    FloatString : FnOnce(f32, Rc<String>)           -> Result<T>,
    StringInt   : FnOnce(Rc<String>, i32)           -> Result<T>,
    StringFloat : FnOnce(Rc<String>, f32)           -> Result<T>,
    StringString: FnOnce(Rc<String>, Rc<String>)    -> Result<T>,
> Fold2<
    T,
    IntInt,
    IntFloat,
    IntString,
    FloatInt,
    FloatFloat,
    FloatString,
    StringInt,
    StringFloat,
    StringString,
> {
    pub fn apply(self, strings: &StringMap) -> Result<T> {
        match self.left {
            Value::Int(l) => match self.right {
                Value::Int(r) => (self.int_int)(l, r),
                Value::Float(r) => (self.int_float)(l, r),
                Value::String(r) => (self.int_string)(l, r.resolve(strings)?),
                Value::Object(_) => return Err(Error::BadValue(BadValue::Type)),
            }
            Value::Float(l) => match self.right {
                Value::Int(r) => (self.float_int)(l, r),
                Value::Float(r) => (self.float_float)(l, r),
                Value::String(r) => (self.float_string)(l, r.resolve(strings)?),
                Value::Object(_) => return Err(Error::BadValue(BadValue::Type)),
            }
            Value::String(l) => {
                let l = l.resolve(strings)?;
                match self.right {
                    Value::Int(r) => (self.string_int)(l, r),
                    Value::Float(r) => (self.string_float)(l, r),
                    Value::String(r) => (self.string_string)(l, r.resolve(strings)?),
                    Value::Object(_) => return Err(Error::BadValue(BadValue::Type)),
                }
            }
            Value::Object(_) => return Err(Error::BadValue(BadValue::Type)),
        }
    }
}