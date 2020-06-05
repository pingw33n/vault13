pub mod db;

use std::num::NonZeroU32;

/// Program ID is the identifier of script bytecode file in `scripts.lst`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProgramId(NonZeroU32);

impl ProgramId {
    pub fn new(val: u32) -> Option<Self> {
        NonZeroU32::new(val).map(Self)
    }

    pub fn index(self) -> usize {
        self.val() as usize - 1
    }

    pub fn val(self) -> u32 {
        self.0.get()
    }
}