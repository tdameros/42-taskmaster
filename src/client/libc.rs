use std::os::raw::{c_int, c_ulong, c_ushort};

pub const ICANON: c_ulong = 0x00000100;
pub const ECHO: c_ulong = 0x00000008;
pub const TCSANOW: c_int = 0;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Termios {
    pub c_iflag: c_ulong,
    pub c_oflag: c_ulong,
    pub c_cflag: c_ulong,
    pub c_lflag: c_ulong,
    pub c_cc: [c_ushort; 32],
    pub c_ispeed: c_ulong,
    pub c_ospeed: c_ulong,
}

extern "C" {
    pub fn tcgetattr(fd: c_int, termios_p: *mut Termios) -> c_int;
    pub fn tcsetattr(fd: c_int, optional_actions: c_int, termios_p: *const Termios) -> c_int;
}
