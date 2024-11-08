use std::ffi;

use super::{mode_t, passwd, Termios};

// Declare the external C functions
extern "C" {
    pub(super) fn setpwent();
    pub(super) fn getpwent() -> *mut passwd;
    pub(super) fn endpwent();
    pub(super) fn kill(pid: super::pid_t, sig: i32) -> i32;
    pub(super) fn umask(new_mask: mode_t) -> mode_t;
    pub(super) fn tcgetattr(fd: i32, termios_p: *mut Termios) -> i32;
    pub(super) fn tcsetattr(fd: i32, optional_actions: i32, termios_p: *const Termios) -> i32;
}
