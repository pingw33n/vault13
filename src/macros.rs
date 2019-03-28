/// For use in const expressions. Otherwise use `EnumExt::len()`.
macro_rules! enum_len {
    ($ty:ty) => {{
        <$ty as ::enum_map::Enum<()>>::POSSIBLE_VALUES
    }};
}