/// For use in const expressions. Otherwise use `EnumExt::len()`.
macro_rules! enum_len {
    ($ty:ty) => {{
        <$ty as ::enum_map::Enum<()>>::POSSIBLE_VALUES
    }};
}

macro_rules! unwrap_or_return {
    ($expression:expr, $pattern:pat => $res:expr) => {
        match $expression {
            $pattern => $res,
            _ => return,
        }
    }
}