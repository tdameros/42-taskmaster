/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use super::{mode_t, Passwd, Termios};

/* -------------------------------------------------------------------------- */
/*                        External Function Declaration                       */
/* -------------------------------------------------------------------------- */
// Declare the external C functions
extern "C" {
    pub(super) fn setpwent();
    pub(super) fn getpwent() -> *mut Passwd;
    pub(super) fn endpwent();
    pub(super) fn kill(pid: super::pid_t, sig: super::c_int) -> super::c_int;
    pub(super) fn umask(new_mask: mode_t) -> mode_t;
    pub(super) fn tcgetattr(fd: super::c_int, termios_p: *mut Termios) -> super::c_int;
    pub(super) fn tcsetattr(
        fd: super::c_int,
        optional_actions: super::c_int,
        termios_p: *const Termios,
    ) -> super::c_int;
    pub(super) fn getppid() -> super::pid_t;
    pub(super) fn signal(signum: super::c_int, handler: super::sighandler_t);
    pub(super) fn pthread_sigmask(
        how: super::c_int,
        set: *const super::sigset_t,
        oldset: *mut super::sigset_t,
    ) -> super::c_int;
    pub(super) fn sigaddset(set: *mut super::sigset_t, signumL: super::c_int) -> super::c_int;
}

