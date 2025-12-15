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
                 Eq, Hash, PartialEq, Ord, PartialOrd,
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

        unsafe impl ::slotmap::Key for $name {
            fn data(&self) -> ::slotmap::KeyData {
                self.0.data()
            }
        }

        )*
    };
}
