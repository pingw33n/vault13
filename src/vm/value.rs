use bstring::BString;
use bstring::bfmt::ToBString;
use std::cmp::Ordering;
use std::rc::Rc;

use super::{BadValue, Error, Result, StringMap};
use crate::game::object::Handle;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ValueKind {
    Int,
    Float,
    String,
    Object,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(i32),
    Float(f32),
    String(StringValue),
    Object(Option<Handle>),
}

impl Value {
    pub fn kind(&self) -> ValueKind {
        match self {
            Value::Int(_) => ValueKind::Int,
            Value::Float(_) => ValueKind::Float,
            Value::String(_) => ValueKind::String,
            Value::Object(_) => ValueKind::Object,
        }
    }

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

    pub fn into_string(self, strings: &StringMap) -> Result<Rc<BString>> {
        self.into_string_value()?.resolve(strings)
    }

    pub fn into_object(self) -> Result<Option<Handle>> {
        if let Value::Object(v) = self {
            Ok(v)
        } else {
            Err(Error::BadValue(BadValue::Type))
        }
    }

    pub fn coerce_into_float(self) -> Result<f32> {
        Ok(match self {
            Value::Int(v) => v as f32,
            Value::Float(v) => v,
            | Value::String(_)
            | Value::Object(_)
            => return Err(Error::BadValue(BadValue::Type)),
        })
    }

    pub fn coerce_into_int(self) -> Result<i32> {
        Ok(match self {
            Value::Int(v) => v,
            Value::Float(v) => v as i32,
            | Value::String(_)
            | Value::Object(_)
            => return Err(Error::BadValue(BadValue::Type)),
        })
    }

    pub fn coerce_into_string(self, strings: &StringMap) -> Result<Rc<BString>> {
        Ok(match self {
            Value::Int(v) => Rc::new(v.to_bstring()),
            Value::Float(v) => Rc::new(format!("{:.5}", v).into()),
            Value::String(v) => v.resolve(strings)?,
            Value::Object(_) => return Err(Error::BadValue(BadValue::Type)),
        })
    }

    pub fn coerce_into_object(self) -> Result<Option<Handle>> {
        if self == Value::Int(0) {
            Ok(None)
        } else {
            self.into_object()
        }
    }

    pub fn coerce_into_same_kind(self, other: Self, strings: &StringMap) -> Result<(Self, Self)> {
        let k1 = self.kind();
        let k2 = other.kind();

        if k1 == k2 {
            return Ok((self, other));
        }

        let (v1, v2) = if k1 == ValueKind::String || k2 == ValueKind::String {
            (self.coerce_into_string(strings)?.into(),
                other.coerce_into_string(strings)?.into())
        } else if k1 == ValueKind::Object || k2 == ValueKind::Object {
            (self.coerce_into_object()?.into(),
                other.coerce_into_object()?.into())
        } else if k1 == ValueKind::Float || k2 == ValueKind::Float {
            (self.coerce_into_float()?.into(),
                other.coerce_into_float()?.into())
        } else {
            unreachable!();
        };

        Ok((v1, v2))
    }

    pub fn resolved(self, strings: &StringMap) -> Result<Value> {
        Ok(match self {
            Value::String(v) => Value::String(v.resolved(strings)?),
            v @ _ => v,
        })
    }

    pub fn test(&self) -> bool {
        match self {
            Value::Int(v) => *v != 0,
            Value::Float(v) => *v != 0.0,
            Value::String(_) => true,
            Value::Object(v) => v.is_some(),
        }
    }

    pub fn not(&self) -> Value {
        Value::from(!self.test())
    }

    pub fn partial_cmp(&self, other: &Value, strings: &StringMap) -> Result<Option<Ordering>> {
        match self.clone().coerce_into_same_kind_and(other.clone(), strings,
            |l, r| Ok(l.partial_cmp(&r)),
            |l, r| Ok(l.partial_cmp(&r)),
            |l, r| Ok(l.partial_cmp(&r)),
            |l, r| Ok(l.partial_cmp(&r)))
        {
            Ok(r) => Ok(r),
            Err(Error::BadValue(BadValue::Type)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn neg(&self) -> Result<Value> {
        match self {
            Value::Int(v)       => Ok(Value::Int(-*v)),
            Value::Float(v)     => Ok(Value::Float(-*v)),
            | Value::String(_)
            | Value::Object(_)
            => Err(Error::BadValue(BadValue::Type)),
        }
    }

    pub fn add(self, other: Value, strings: &StringMap) -> Result<Value> {
        self.coerce_into_same_kind_and(other, strings,
            |l, r| Ok((l + r).into()),
            |l, r| Ok((l + r).into()),
            |l, r| Ok(BString::concat(&[l.as_bstr(), r.as_bstr()]).into()),
            |_, _| Err(Error::BadValue(BadValue::Type)),
        )
    }

    pub fn div(self, other: Value, strings: &StringMap) -> Result<Value> {
        let lkind_bad = match self.kind() {
            ValueKind::Int | ValueKind::Float => false,
            _ => true
        };
        let rkind_bad = match other.kind() {
            ValueKind::Int | ValueKind::Float => false,
            _ => true
        };
        if lkind_bad || rkind_bad {
            return Err(Error::BadValue(BadValue::Type));
        }
        self.coerce_into_same_kind_and(other, strings,
            |l, r| l.checked_div(r).map(|v| v.into()).ok_or(Error::BadValue(BadValue::Content)),
            |l, r| if r == 0.0 {
                Err(Error::BadValue(BadValue::Content))
            } else {
                Ok((l / r).into())
            },
            |_, _| unreachable!(),
            |_, _| unreachable!(),
        )
    }

    pub fn mul(self, other: Value, strings: &StringMap) -> Result<Value> {
        let lkind_bad = match self.kind() {
            ValueKind::Int | ValueKind::Float => false,
            _ => true
        };
        let rkind_bad = match other.kind() {
            ValueKind::Int | ValueKind::Float => false,
            _ => true
        };
        if lkind_bad || rkind_bad {
            return Err(Error::BadValue(BadValue::Type));
        }
        self.coerce_into_same_kind_and(other, strings,
            |l, r| l.checked_mul(r).map(|v| v.into()).ok_or(Error::BadValue(BadValue::Content)),
            |l, r| Ok((l * r).into()),
            |_, _| unreachable!(),
            |_, _| unreachable!(),
        )
    }

    pub fn rem(self, other: Value, _strings: &StringMap) -> Result<Value> {
        let l = self.into_int()?;
        let r = other.into_int()?;
        l.checked_rem(r).map(|v| v.into()).ok_or(Error::BadValue(BadValue::Content))
    }

    pub fn sub(self, other: Value, strings: &StringMap) -> Result<Value> {
        let lkind_bad = match self.kind() {
            ValueKind::Int | ValueKind::Float => false,
            _ => true
        };
        let rkind_bad = match other.kind() {
            ValueKind::Int | ValueKind::Float => false,
            _ => true
        };
        if lkind_bad || rkind_bad {
            return Err(Error::BadValue(BadValue::Type));
        }
        self.coerce_into_same_kind_and(other, strings,
            |l, r| l.checked_sub(r).map(|v| v.into()).ok_or(Error::BadValue(BadValue::Content)),
            |l, r| Ok((l - r).into()),
            |_, _| unreachable!(),
            |_, _| unreachable!(),
        )
    }

    pub fn bwand(self, other: Value) -> Result<Value> {
        let left = self.coerce_into_int()?;
        let right = other.coerce_into_int()?;
        Ok((left & right).into())
    }

    pub fn bwnot(self) -> Result<Value> {
        let v = self.coerce_into_int()?;
        Ok((!v).into())
    }

    pub fn bwor(self, other: Value) -> Result<Value> {
        let left = self.coerce_into_int()?;
        let right = other.coerce_into_int()?;
        Ok((left | right).into())
    }

    pub fn bwxor(self, other: Value) -> Result<Value> {
        let left = self.coerce_into_int()?;
        let right = other.coerce_into_int()?;
        Ok((left ^ right).into())
    }

    fn coerce_into_same_kind_and<
        T,
        Ints,
        Floats,
        Strings,
        Objects,
    >(self, other: Self, string_map: &StringMap,
        ints    : Ints,
        floats  : Floats,
        strings : Strings,
        objects : Objects,
    ) -> Result<T>
        where
            Ints    : FnOnce(i32, i32)                          -> Result<T>,
            Floats  : FnOnce(f32, f32)                          -> Result<T>,
            Strings : FnOnce(Rc<BString>, Rc<BString>)            -> Result<T>,
            Objects : FnOnce(Option<Handle>, Option<Handle>)    -> Result<T>,
    {
        let (left, right) = self.coerce_into_same_kind(other, string_map)?;
        match left {
             Value::Int(l) => ints(l, right.into_int().unwrap()),
             Value::Float(l) => floats(l, right.into_float().unwrap()),
             Value::String(l) => strings(l.resolve(string_map)?,
                 right.into_string(string_map).unwrap()),
             Value::Object(l) => objects(l, right.into_object().unwrap()),
         }
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

impl From<Rc<BString>> for Value {
    fn from(v: Rc<BString>) -> Self {
        Value::String(StringValue::Direct(v))
    }
}

impl From<BString> for Value {
    fn from(v: BString) -> Self {
        Value::String(StringValue::Direct(Rc::new(v)))
    }
}

impl<'a> From<&'a str> for Value {
    fn from(v: &'a str) -> Self {
        Self::from(v.to_bstring())
    }
}

impl From<Option<Handle>> for Value {
    fn from(v: Option<Handle>) -> Self {
        Value::Object(v)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StringValue {
    Indirect(usize),
    Direct(Rc<BString>),
}

impl StringValue {
    pub fn resolve(self, strings: &StringMap) -> Result<Rc<BString>> {
        Ok(self.resolved(strings)?.into_direct().unwrap())

    }
    pub fn resolved(self, strings: &StringMap) -> Result<StringValue> {
        Ok(match self {
            StringValue::Indirect(id) => StringValue::Direct(
                strings.get(id).ok_or(Error::BadValue(BadValue::Content))?.clone()),
            StringValue::Direct(s) => StringValue::Direct(s),
        })
    }

    pub fn into_direct(self) -> Option<Rc<BString>> {
        if let StringValue::Direct(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use self::Value::*;
    use self::StringValue::*;

    fn bad_type<T>() -> Result<T> {
        Err(Error::BadValue(BadValue::Type))
    }

    fn strings(d: &[(usize, &'static str)]) -> StringMap {
        let mut strings = StringMap::new();
        for &(id, s) in d {
            strings.insert(id, Rc::new(s.into()));
        }
        strings
    }

    // Generates and appends String(Direct(...)) cases for resolvable String(Indirect(...)) ones.
    fn generate_string_direct_cases<T: Clone>(d: &mut Vec<(Value, Value, T)>, strings: &StringMap) {
        for i in 0..d.len() {
            let (left, right, exp) = d[i].clone();
            if let String(Indirect(left_id)) = left {
                if let Some(left) = strings.get(left_id).cloned() {
                    d.push((left.into(), right, exp));
                }
            }
        }
        for i in 0..d.len() {
            let (left, right, exp) = d[i].clone();
            if let String(Indirect(right_id)) = right {
                if let Some(right) = strings.get(right_id).cloned() {
                    d.push((left, right.into(), exp));
                }
            }
        }
    }

    fn value_variants(string_id: usize) -> Vec<Value> {
        vec![
            Int(0),
            Int(i32::max_value()),
            Int(i32::min_value()),
            Float(0.0),
            Float(-0.0),
            Float(::std::f32::MIN),
            Float(::std::f32::MAX),
            String(Indirect(string_id)),
            String(Direct(Rc::new("".into()))),
            String(Direct(Rc::new("test".into()))),
            String(Direct(Rc::new("123".into()))),
            String(Direct(Rc::new("-123".into()))),
            String(Direct(Rc::new("0.0".into()))),
            Object(None),
            Object(Some(Handle::null())),
        ]
    }

    fn fill_bad_type_binary_variants<T>(d: &mut Vec<(Value, Value, Result<T>)>, string_id: usize,
            commutative: bool) {
        let exclude: Vec<_> = d.iter()
            .filter(|e| e.2.is_ok())
            .flat_map(|e| {
                let mut r = vec![(e.0.kind(), e.1.kind())];
                if commutative {
                    r.push((e.1.kind(), e.0.kind()));
                }
                r
            })
            .collect();
        let vars = value_variants(string_id);
        for left in &vars {
            for right in &vars {
                if exclude.iter().any(|e| e == &(left.kind(), right.kind())) {
                    continue;
                }
                d.push((left.clone(), right.clone(), bad_type()));
            }
        }
    }

    fn fill_bad_type_unary_variants<T>(d: &mut Vec<(Value, Result<T>)>, string_id: usize) {
        let exclude: Vec<_> = d.iter()
            .filter(|e| e.1.is_ok())
            .map(|e| e.0.kind())
            .collect();
        let vars = value_variants(string_id);
        for v in &vars {
            if exclude.iter().any(|e| e == &v.kind()) {
                continue;
            }
            d.push((v.clone(), bad_type()));
        }
    }

    #[test]
    fn coerce_into_float() {
        let mut d = vec![
            (Int(0), Ok(0.0)),
            (Int(123), Ok(123.0)),
            (Float(0.0), Ok(0.0)),
            (Float(12.3), Ok(12.3)),
        ];
        fill_bad_type_unary_variants(&mut d, 0);
        for (inp, exp) in d {
            assert_eq!(inp.coerce_into_float(), exp);
        }
    }

    #[test]
    fn coerce_into_string() {
        const S: &'static [(usize, &'static str)] = &[
            (12, "s1"),    // 0
        ];
        let ref strings = strings(S);
        let mut d = vec![
            (Int(0), Ok("0")),
            (Int(123), Ok("123")),
            (Float(0.0), Ok("0.00000")),
            (Float(12.3), Ok("12.30000")),
            (String(Indirect(S[0].0)), Ok("s1")),
            (String(Direct(Rc::new("ds1".into()))), Ok("ds1")),
            (String(Indirect(12345678)), Err(Error::BadValue(BadValue::Content))),
        ];
        fill_bad_type_unary_variants(&mut d, 0);
        for (inp, exp) in d {
            assert_eq!(inp.coerce_into_string(strings), exp.map(|v| Rc::new(v.into())));
        }
    }

    #[test]
    fn coerce_into_object() {
        let mut d = vec![
            (Int(0), Ok(None)),
            (Object(None), Ok(None)),
            (Object(Some(Handle::null())), Ok(Some(Handle::null()))),
        ];
        fill_bad_type_unary_variants(&mut d, 0);
        for (inp, exp) in d {
            assert_eq!(inp.coerce_into_object(), exp);
        }
    }

    #[test]
    fn neg() {
        let mut d = vec![
            (Int(0), Ok(Int(0))),
            (Int(1), Ok(Int(-1))),
            (Int(-1), Ok(Int(1))),
            (Float(0.0), Ok(Float(-0.0))),
            (Float(1.0), Ok(Float(-1.0))),
            (Float(-0.0), Ok(Float(0.0))),
            (Float(-1.0), Ok(Float(1.0))),
            (String(Indirect(usize::max_value())), bad_type()),
        ];
        fill_bad_type_unary_variants(&mut d, 0);
        for (inp, exp) in d {
            assert_eq!(inp.neg(), exp);
        }
    }

    #[test]
    fn test_and_not() {
        let d = vec![
            (Int(0), false),
            (Int(1), true),
            (Int(-1), true),
            (Float(0.0), false),
            (Float(1.0), true),
            (Float(-0.0), false),
            (Float(-1.0), true),
            (String(Indirect(0)), true),
            (String(Indirect(1)), true),
            (String(Indirect(usize::max_value())), true),
            (String(Direct(Rc::new("".into()))), true),
            (String(Direct(Rc::new("123".into()))), true),
            (Object(None), false),
            (Object(Some(Handle::null())), true),
        ];
        for (inp, exp) in d {
            assert_eq!(inp.test(), exp, "{:?} {:?}", inp, exp);
            assert_eq!(inp.not(), Value::from(!exp));
        }
    }

    #[test]
    fn partial_cmp() {
        use std::cmp::Ordering::*;
        use std::f32;

        const S: &'static [(usize, &'static str)] = &[
            (12, "string 1"),       // 0
            (34, "string 1"),       // 1
            (56, "string 2"),       // 2
            (78, "STRING 2"),       // 3
            (100, "0"),             // 4
            (101, "1"),             // 5
            (102, "10"),            // 6
            (103, "0.00000"),       // 7
            (104, "12.12300"),      // 8
            (105, "-12.12300"),     // 9
            (106, ""),              // 10
        ];
        let strings = strings(S);
        let mut d = vec![
            // Same types.

            (Int(0), Int(0), Ok(Some(Equal))),
            (Int(i32::min_value()), Int(i32::min_value()), Ok(Some(Equal))),
            (Int(i32::max_value()), Int(i32::max_value()), Ok(Some(Equal))),
            (Int(i32::min_value()), Int(i32::max_value()), Ok(Some(Less))),

            (Float(0.0), Float(0.0), Ok(Some(Equal))),
            (Float(-0.0), Float(0.0), Ok(Some(Equal))),
            (Float(0.0), Float(-0.0), Ok(Some(Equal))),
            (Float(f32::MIN), Float(f32::MIN), Ok(Some(Equal))),
            (Float(f32::MAX), Float(f32::MAX), Ok(Some(Equal))),
            (Float(f32::MIN), Float(f32::MAX), Ok(Some(Less))),
            (Float(f32::NAN), Float(0.0), Ok(None)),
            (Float(0.0), Float(f32::NAN), Ok(None)),

            (String(Indirect(S[0].0)), String(Indirect(S[0].0)), Ok(Some(Equal))),
            (String(Indirect(S[0].0)), String(Indirect(S[1].0)), Ok(Some(Equal))),
            (String(Indirect(S[1].0)), String(Indirect(S[0].0)), Ok(Some(Equal))),
            (String(Indirect(S[1].0)), String(Indirect(S[2].0)), Ok(Some(Less))),
            (String(Indirect(S[0].0)), String(Indirect(S[2].0)), Ok(Some(Less))),
            (String(Indirect(S[3].0)), String(Indirect(S[2].0)), Ok(Some(Less))),
            // String(Direct(...)) cases are generated by the code.

            (Object(None), Object(None), Ok(Some(Equal))),
            (Object(Some(Handle::null())), Object(Some(Handle::null())), Ok(Some(Equal))),
            (Object(None), Object(Some(Handle::null())), Ok(Some(Less))),

            // Mixed types.

            (Int(0), Float(0.0), Ok(Some(Equal))),
            (Int(0), Float(-0.0), Ok(Some(Equal))),
            (Int(0), Float(1.0), Ok(Some(Less))),
            (Int(0), Float(f32::NAN), Ok(None)),
            (Int(0), String(Indirect(S[4].0)), Ok(Some(Equal))),
            (Int(0), String(Indirect(S[5].0)), Ok(Some(Less))),
            (Int(2), String(Indirect(S[6].0)), Ok(Some(Greater))),
            (Int(0), Object(None), Ok(Some(Equal))),
            (Int(0), Object(Some(Handle::null())), Ok(Some(Less))),
            (Int(1), Object(None), Ok(None)),
            (Int(1), Object(Some(Handle::null())), Ok(None)),
            (Float(0.0), String(Indirect(S[7].0)), Ok(Some(Equal))),
            (Float(12.123), String(Indirect(S[8].0)), Ok(Some(Equal))),
            (Float(-12.123), String(Indirect(S[9].0)), Ok(Some(Equal))),
            (Float(0.0), String(Indirect(S[8].0)), Ok(Some(Less))),
            (Float(0.0), String(Indirect(S[9].0)), Ok(Some(Greater))),
            (String(Indirect(S[0].0)), Object(None), Ok(None)),
            (String(Indirect(S[0].0)), Object(Some(Handle::null())), Ok(None)),
            (String(Indirect(S[10].0)), Object(None), Ok(None)),
            (String(Indirect(S[10].0)), Object(Some(Handle::null())), Ok(None)),

            (String(Indirect(12345678)), Int(0), Err(Error::BadValue(BadValue::Content))),
        ];
        generate_string_direct_cases(&mut d, &strings);

        for (left, right, exp) in d {
            assert_eq!(left.partial_cmp(&right, &strings), exp,
                "{:?} {:?} {:?}", left, right, exp);
            let exp_rev = exp.map(|v| v.map(|v| v.reverse()));
            assert_eq!(right.partial_cmp(&left, &strings), exp_rev,
                "{:?} {:?} {:?}", left, right, exp_rev);
        }
    }

    #[test]
    fn add() {
        const S: &'static [(usize, &'static str)] = &[
            (12, "s1_"),     // 0
            (34, "S2"),     // 1
        ];
        let strings = strings(S);
        let mut d = vec![
            (Int(123), Int(456), Ok(Int(123 + 456))),
            (Int(123), Float(456.789), Ok(Float(123 as f32 + 456.789))),
            (Float(123.456), Float(456.789), Ok(Float(123.456 + 456.789))),
            (String(Indirect(S[0].0)), String(Indirect(S[1].0)), Ok("s1_S2".into())),
            (String(Indirect(S[0].0)), Int(42), Ok("s1_42".into())),
            (Int(42), String(Indirect(S[0].0)), Ok("42s1_".into())),
            (String(Indirect(S[0].0)), Float(12.123), Ok("s1_12.12300".into())),
            (Float(12.123), String(Indirect(S[0].0)), Ok("12.12300s1_".into())),

            (String(Indirect(usize::max_value())), Int(0), Err(Error::BadValue(BadValue::Content))),
        ];
        generate_string_direct_cases(&mut d, &strings);
        fill_bad_type_binary_variants(&mut d, S[0].0, true);

        for (left, right, exp) in d {
            assert_eq!(left.clone().add(right.clone(), &strings), exp,
                "{:?} {:?} {:?}", left, right, exp);
            if left.kind() != ValueKind::String && right.kind() != ValueKind::String
                    || exp.is_err() {
                assert_eq!(right.clone().add(left.clone(), &strings), exp,
                    "{:?} {:?} {:?}", left, right, exp);
            }
        }
    }

    #[test]
    fn div() {
        const S: &'static [(usize, &'static str)] = &[
            (12, "123"),     // 0
            (34, "123.456"), // 1
            (56, "0"),       // 2
            (56, "0.0"),     // 3
        ];
        let strings = strings(S);

        let mut d = vec![
            (Int(456), Int(123), Ok(Int(456 / 123))),
            (Int(123), Float(456.789), Ok(Float(123.0 / 456.789))),
            (Float(123.456), Float(456.789), Ok(Float(123.456 / 456.789))),

            (Int(456), Int(0), Err(Error::BadValue(BadValue::Content))),
            (Int(i32::min_value()), Int(-1), Err(Error::BadValue(BadValue::Content))),

            (String(Indirect(S[0].0)), String(Indirect(S[1].0)), Err(Error::BadValue(BadValue::Type))),
            (String(Indirect(S[0].0)), Int(42), Err(Error::BadValue(BadValue::Type))),
            (String(Indirect(S[2].0)), Int(42), Err(Error::BadValue(BadValue::Type))),
            (String(Indirect(S[3].0)), Int(42), Err(Error::BadValue(BadValue::Type))),
            (Int(42), String(Indirect(S[0].0)), Err(Error::BadValue(BadValue::Type))),
            (Int(42), String(Indirect(S[2].0)), Err(Error::BadValue(BadValue::Type))),
            (Int(42), String(Indirect(S[3].0)), Err(Error::BadValue(BadValue::Type))),
            (String(Indirect(S[0].0)), Float(12.123), Err(Error::BadValue(BadValue::Type))),
            (Float(12.123), String(Indirect(S[0].0)), Err(Error::BadValue(BadValue::Type))),
            (String(Indirect(usize::max_value())), Int(0), Err(Error::BadValue(BadValue::Type))),
        ];
        generate_string_direct_cases(&mut d, &strings);

        for (left, right, exp) in d {
            assert_eq!(left.clone().div(right.clone(), &strings), exp,
                "{:?} {:?} {:?}", left, right, exp);
        }
    }

    #[test]
    fn mul() {
        const S: &'static [(usize, &'static str)] = &[
            (12, "123"),     // 0
            (34, "123.456"), // 1
            (56, "0"),       // 2
            (56, "0.0"),     // 3
        ];
        let strings = strings(S);

        let mut d = vec![
            (Int(456), Int(0), Ok(Int(456 * 0))),
            (Int(456), Int(123), Ok(Int(456 * 123))),
            (Int(123), Float(456.789), Ok(Float(123.0 * 456.789))),
            (Float(123.456), Float(456.789), Ok(Float(123.456 * 456.789))),

            (Int(i32::min_value()), Int(-1), Err(Error::BadValue(BadValue::Content))),
            (Int(i32::max_value()), Int(2), Err(Error::BadValue(BadValue::Content))),

            (String(Indirect(S[0].0)), String(Indirect(S[1].0)), Err(Error::BadValue(BadValue::Type))),
            (String(Indirect(S[0].0)), Int(42), Err(Error::BadValue(BadValue::Type))),
            (String(Indirect(S[2].0)), Int(42), Err(Error::BadValue(BadValue::Type))),
            (String(Indirect(S[3].0)), Int(42), Err(Error::BadValue(BadValue::Type))),
            (Int(42), String(Indirect(S[0].0)), Err(Error::BadValue(BadValue::Type))),
            (Int(42), String(Indirect(S[2].0)), Err(Error::BadValue(BadValue::Type))),
            (Int(42), String(Indirect(S[3].0)), Err(Error::BadValue(BadValue::Type))),
            (String(Indirect(S[0].0)), Float(12.123), Err(Error::BadValue(BadValue::Type))),
            (Float(12.123), String(Indirect(S[0].0)), Err(Error::BadValue(BadValue::Type))),
            (String(Indirect(usize::max_value())), Int(0), Err(Error::BadValue(BadValue::Type))),
        ];
        generate_string_direct_cases(&mut d, &strings);
        fill_bad_type_binary_variants(&mut d, S[0].0, true);

        for (left, right, exp) in d {
            assert_eq!(left.clone().mul(right.clone(), &strings), exp,
                "{:?} {:?} {:?}", left, right, exp);
            if exp.is_err() {
                assert_eq!(right.clone().mul(left.clone(), &strings), exp,
                    "{:?} {:?} {:?}", left, right, exp);
            }
        }
    }

    #[test]
    fn rem() {
        const S: &'static [(usize, &'static str)] = &[
            (12, "123"),     // 0
            (34, "123.456"), // 1
            (56, "0"),       // 2
            (56, "0.0"),     // 3
        ];
        let strings = strings(S);

        let mut d = vec![
            (Int(456), Int(123), Ok(Int(456 % 123))),

            (Int(456), Int(0), Err(Error::BadValue(BadValue::Content))),
            (Int(i32::min_value()), Int(-1), Err(Error::BadValue(BadValue::Content))),

            (Int(123), Float(456.789), Err(Error::BadValue(BadValue::Type))),
            (Float(123.456), Float(456.789), Err(Error::BadValue(BadValue::Type))),

            (String(Indirect(S[0].0)), String(Indirect(S[1].0)), Err(Error::BadValue(BadValue::Type))),
            (String(Indirect(S[0].0)), Int(42), Err(Error::BadValue(BadValue::Type))),
            (String(Indirect(S[2].0)), Int(42), Err(Error::BadValue(BadValue::Type))),
            (String(Indirect(S[3].0)), Int(42), Err(Error::BadValue(BadValue::Type))),
            (Int(42), String(Indirect(S[0].0)), Err(Error::BadValue(BadValue::Type))),
            (Int(42), String(Indirect(S[2].0)), Err(Error::BadValue(BadValue::Type))),
            (Int(42), String(Indirect(S[3].0)), Err(Error::BadValue(BadValue::Type))),
            (String(Indirect(S[0].0)), Float(12.123), Err(Error::BadValue(BadValue::Type))),
            (Float(12.123), String(Indirect(S[0].0)), Err(Error::BadValue(BadValue::Type))),
            (String(Indirect(usize::max_value())), Int(0), Err(Error::BadValue(BadValue::Type))),
        ];
        generate_string_direct_cases(&mut d, &strings);

        for (left, right, exp) in d {
            assert_eq!(left.clone().rem(right.clone(), &strings), exp,
                "{:?} {:?} {:?}", left, right, exp);
        }
    }

    #[test]
    fn sub() {
        const S: &'static [(usize, &'static str)] = &[
            (12, "123"),     // 0
            (34, "123.456"), // 1
            (56, "0"),       // 2
            (56, "0.0"),     // 3
        ];
        let strings = strings(S);

        let mut d = vec![
            (Int(123), Int(456), Ok(Int(123 - 456))),
            (Int(123), Float(456.789), Ok(Float(123.0 - 456.789))),
            (Float(123.456), Float(456.789), Ok(Float(123.456 - 456.789))),

            (Int(i32::min_value()), Int(1), Err(Error::BadValue(BadValue::Content))),

            (String(Indirect(S[0].0)), String(Indirect(S[1].0)), Err(Error::BadValue(BadValue::Type))),
            (String(Indirect(S[0].0)), Int(42), Err(Error::BadValue(BadValue::Type))),
            (String(Indirect(S[2].0)), Int(42), Err(Error::BadValue(BadValue::Type))),
            (String(Indirect(S[3].0)), Int(42), Err(Error::BadValue(BadValue::Type))),
            (Int(42), String(Indirect(S[0].0)), Err(Error::BadValue(BadValue::Type))),
            (Int(42), String(Indirect(S[2].0)), Err(Error::BadValue(BadValue::Type))),
            (Int(42), String(Indirect(S[3].0)), Err(Error::BadValue(BadValue::Type))),
            (String(Indirect(S[0].0)), Float(12.123), Err(Error::BadValue(BadValue::Type))),
            (Float(12.123), String(Indirect(S[0].0)), Err(Error::BadValue(BadValue::Type))),
            (String(Indirect(usize::max_value())), Int(0), Err(Error::BadValue(BadValue::Type))),
        ];
        generate_string_direct_cases(&mut d, &strings);

        for (left, right, exp) in d {
            assert_eq!(left.clone().sub(right.clone(), &strings), exp,
                "{:?} {:?} {:?}", left, right, exp);
        }
    }

    #[test]
    fn bitwise_binary() {
        let mut d = vec![
            (Int(0b11100), Int(0b00111),
                // bwand        bwor        bwxor       bwnot
                Ok((0b00100,    0b11111,    0b11011,    -29))),
            (Int(-256), Int(0xfff),
                Ok((0xf00,      -1,         -3841,      0xff))),
            (String(Indirect(12345678)), Int(0),
                Err(Error::BadValue(BadValue::Type))),
        ];
        for i in 0..d.len() {
            let (left, right, exp) = d[i].clone();
            if let Int(v) = left {
                d.push((Value::Float(v as f32), right, exp));
            }
        }
        for i in 0..d.len() {
            let (left, right, exp) = d[i].clone();
            if let Int(v) = right {
                d.push((left, Value::Float(v as f32), exp));
            }
        }
        fill_bad_type_binary_variants(&mut d, 0, true);

        for (left, right, exp) in d {
            let bwand = exp.clone().map(|v| v.0.into());
            assert_eq!(left.clone().bwand(right.clone()), bwand,
                "{:?} {:?} {:?}", left, right, bwand);
            assert_eq!(right.clone().bwand(left.clone()), bwand,
                "{:?} {:?} {:?}", right, left, bwand);

            let bwor = exp.clone().map(|v| v.1.into());
            assert_eq!(left.clone().bwor(right.clone()), bwor,
                "{:?} {:?} {:?}", left, right, bwor);
            assert_eq!(right.clone().bwor(left.clone()), bwor,
                "{:?} {:?} {:?}", right, left, bwor);

            let bwxor = exp.clone().map(|v| v.2.into());
            assert_eq!(left.clone().bwxor(right.clone()), bwxor,
                "{:?} {:?} {:?}", left, right, bwxor);
            assert_eq!(right.clone().bwxor(left.clone()), bwxor,
                "{:?} {:?} {:?}", right, left, bwxor);
        }
    }

    #[test]
    fn bitwise_unary() {
        let mut d = vec![
            (Int(0b11100), Ok(-29)),
            (Int(-256), Ok(0xff)),
            (String(Indirect(12345678)), Err(Error::BadValue(BadValue::Type))),
        ];
        for i in 0..d.len() {
            let (v, exp) = d[i].clone();
            if let Int(v) = v {
                d.push((Value::Float(v as f32), exp));
            }
        }
        fill_bad_type_unary_variants(&mut d, 0);

        for (v, exp) in d {
            let exp = exp.clone().map(|v| v.into());
            assert_eq!(v.clone().bwnot(), exp.clone(),
                "{:?} {:?}", v, exp);

            if let Ok(exp) = exp {
                let new_exp = Ok(Value::Int(v.clone().coerce_into_int().unwrap()));
                assert_eq!(exp.clone().bwnot(), new_exp.clone(),
                    "{:?} {:?}", exp, new_exp);
            }
        }
    }
}