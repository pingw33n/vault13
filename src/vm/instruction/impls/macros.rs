macro_rules! log_ {
    ($vm_state:expr) => {
        log::debug!("[0x{:06x}] {:?}",
            ($vm_state).opcode.unwrap().1,
            ($vm_state).opcode.unwrap().0);
    };
}

macro_rules! log_a1 {
    ($vm_state:expr, $arg:expr) => {
        log::debug!("[0x{:06x}] {:?} ({:?})",
            ($vm_state).opcode.unwrap().1,
            ($vm_state).opcode.unwrap().0,
            $arg);
    }
}

macro_rules! log_a1r1 {
    ($vm_state:expr, $arg:expr, $res:expr) => {
        log::debug!("[0x{:06x}] {:?} ({:?}) -> ({:?})",
            ($vm_state).opcode.unwrap().1,
            ($vm_state).opcode.unwrap().0,
            $arg, $res);
    }
}

macro_rules! log_a1r2 {
    ($vm_state:expr, $arg:expr, $res1:expr, $res2:expr) => {
        log::debug!("[0x{:06x}] {:?} ({:?}) -> ({:?}, {:?})",
            ($vm_state).opcode.unwrap().1,
            ($vm_state).opcode.unwrap().0,
            $arg, $res1, $res2);
    }
}

macro_rules! log_a2 {
    ($vm_state:expr, $arg1:expr, $arg2:expr) => {
        log::debug!("[0x{:06x}] {:?} ({:?}, {:?})",
            ($vm_state).opcode.unwrap().1,
            ($vm_state).opcode.unwrap().0,
            $arg1, $arg2);
    }
}

macro_rules! log_a2r1 {
    ($vm_state:expr, $arg1:expr, $arg2:expr, $res:expr) => {
        log::debug!("[0x{:06x}] {:?} ({:?}, {:?}) -> ({:?})",
            ($vm_state).opcode.unwrap().1,
            ($vm_state).opcode.unwrap().0,
            $arg1, $arg2, $res);
    }
}

macro_rules! log_a3 {
    ($vm_state:expr, $arg1:expr, $arg2:expr, $arg3:expr) => {
        log::debug!("[0x{:06x}] {:?} ({:?}, {:?}, {:?})",
            ($vm_state).opcode.unwrap().1,
            ($vm_state).opcode.unwrap().0,
            $arg1, $arg2, $arg3);
    }
}

macro_rules! log_a3r1 {
    ($vm_state:expr, $arg1:expr, $arg2:expr, $arg3:expr, $res:expr) => {
        log::debug!("[0x{:06x}] {:?} ({:?}, {:?}, {:?}) -> ({:?})",
            ($vm_state).opcode.unwrap().1,
            ($vm_state).opcode.unwrap().0,
            $arg1, $arg2, $arg3, $res);
    }
}

macro_rules! log_a4r1 {
    ($vm_state:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, $res:expr) => {
        log::debug!("[0x{:06x}] {:?} ({:?}, {:?}, {:?}, {:?}) -> ({:?})",
            ($vm_state).opcode.unwrap().1,
            ($vm_state).opcode.unwrap().0,
            $arg1, $arg2, $arg3, $arg4, $res);
    }
}

macro_rules! log_a4 {
    ($vm_state:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr) => {
        log::debug!("[0x{:06x}] {:?} ({:?}, {:?}, {:?}, {:?})",
            ($vm_state).opcode.unwrap().1,
            ($vm_state).opcode.unwrap().0,
            $arg1, $arg2, $arg3, $arg4);
    }
}

macro_rules! log_r1 {
    ($vm_state:expr, $res:expr) => {
        log::debug!("[0x{:06x}] {:?} -> ({:?})",
            ($vm_state).opcode.unwrap().1,
            ($vm_state).opcode.unwrap().0,
            $res);
    }
}

macro_rules! log_stub {
    ($vm_state:expr) => {
        log::warn!("called {:?} which is a noop stub!", ($vm_state).opcode.unwrap().0);
    }
}