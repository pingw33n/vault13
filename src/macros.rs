/// For use in const expressions. Otherwise use `EnumExt::len()`.
macro_rules! enum_len {
    ($ty:ty) => {{
        <$ty as ::enum_map::Enum<()>>::POSSIBLE_VALUES
    }};
}

macro_rules! unwrap_or_return {
    ($expr:expr, $pat:pat => $res:expr) => {
        match $expr {
            $pat => $res,
            _ => return,
        }
    };
    ($expr:expr, Some) => {
        unwrap_or_return!($expr, Some(v) => v)
    };
}

macro_rules! new_handle_type {
    ( $( $(#[$outer:meta])* $vis:vis struct $name:ident; )* ) => {
        $(

        $(#[$outer])*
        #[derive(Copy, Clone, Default,
                 Eq, PartialEq, Ord, PartialOrd,
                 Debug)]
        #[repr(transparent)]
        $vis struct $name($crate::util::SmKey);

        impl From<::slotmap::KeyData> for $name {
            fn from(k: ::slotmap::KeyData) -> Self {
                $name(k.into())
            }
        }

        impl From<$name> for ::slotmap::KeyData {
            fn from(k: $name) -> Self {
                k.0.into()
            }
        }

        impl ::slotmap::Key for $name {}

        )*
    };
}