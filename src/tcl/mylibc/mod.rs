/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use std::{
    ffi::{c_char, c_int, CStr},
    io,
    os::unix::io::AsRawFd,
};

/* -------------------------------------------------------------------------- */
/*                                   Module                                   */
/* -------------------------------------------------------------------------- */
mod raw;

/* -------------------------------------------------------------------------- */
/*                                    Types                                   */
/* -------------------------------------------------------------------------- */
#[allow(non_camel_case_types)]
pub type tcflag_t = u32;
#[allow(non_camel_case_types)]
pub type cc_t = u8;
#[allow(non_camel_case_types)]
pub type speed_t = u32;
#[allow(non_camel_case_types)]
pub type mode_t = u32;
#[allow(non_camel_case_types)]
pub type uid_t = u32;
#[allow(non_camel_case_types)]
pub type gid_t = u32;
#[allow(non_camel_case_types)]
pub type pid_t = i32;
#[allow(non_camel_case_types)]
pub type sighandler_t = extern "C" fn(c_int) -> ();

/* -------------------------------------------------------------------------- */
/*                                  Constants                                 */
/* -------------------------------------------------------------------------- */
// Signals
pub const SIGABRT: c_int = 6;
pub const SIGALRM: c_int = 14;
pub const SIGBUS: c_int = 7;
pub const SIGCHLD: c_int = 17;
pub const SIGCONT: c_int = 18;
pub const SIGFPE: c_int = 8;
pub const SIGHUP: c_int = 1;
pub const SIGILL: c_int = 4;
pub const SIGINT: c_int = 2;
pub const SIGKILL: c_int = 9;
pub const SIGPIPE: c_int = 13;
#[cfg(target_os = "linux")]
pub const SIGPOLL: c_int = 29;
pub const SIGPROF: c_int = 27;
pub const SIGQUIT: c_int = 3;
pub const SIGSEGV: c_int = 11;
pub const SIGSTOP: c_int = 19;
pub const SIGSYS: c_int = 31;
pub const SIGTERM: c_int = 15;
pub const SIGTRAP: c_int = 5;
pub const SIGTSTP: c_int = 20;
pub const SIGTTIN: c_int = 21;
pub const SIGTTOU: c_int = 22;
pub const SIGUSR1: c_int = 10;
pub const SIGUSR2: c_int = 12;
pub const SIGURG: c_int = 23;
pub const SIGVTALRM: c_int = 26;
pub const SIGXCPU: c_int = 24;
pub const SIGXFSZ: c_int = 25;
pub const SIGWINCH: c_int = 28;

// Terminal control flags
pub const NCCS: usize = 32;
pub const ECHO: tcflag_t = 8;
pub const ICANON: tcflag_t = 2;
pub const TCSANOW: c_int = 0;

/* -------------------------------------------------------------------------- */
/*                                   Struct                                   */
/* -------------------------------------------------------------------------- */
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Termios {
    pub c_iflag: tcflag_t,  // Input mode flags
    pub c_oflag: tcflag_t,  // Output mode flags
    pub c_cflag: tcflag_t,  // Control mode flags
    pub c_lflag: tcflag_t,  // Local mode flags
    pub c_line: i8,         // Line discipline
    pub c_cc: [cc_t; NCCS], // Control characters
    pub c_ispeed: speed_t,  // Input speed
    pub c_ospeed: speed_t,  // Output speed
}

#[repr(C)]
pub struct Passwd {
    pub pw_name: *mut c_char,
    pw_passwd: *mut c_char,
    pub pw_uid: uid_t,
    pub pw_gid: gid_t,
    pw_gecos: *mut c_char,
    pw_dir: *mut c_char,
    pw_shell: *mut c_char,
}

#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Debug, Default)]
pub struct sigset_t {
    pub val: [u64; 16],
}

/// represent a way to use the signal present in a sigset_t
/// for more info see the [`pthread_sigmask`] documentation.
#[repr(C)]
#[allow(non_camel_case_types)]
pub enum How {
    /// The set of blocked signals is the union of the current set
    /// and the set argument.
    SIG_BLOCK = 0,
    /// The signals in set are removed from the current set of
    /// blocked signals. It is permissible to attempt to unblock
    /// a signal which is not blocked.
    SIG_UNBLOCK = 1,
    /// The set of blocked signals is set to the argument set.
    SIG_SETMASK = 2,
}

/* -------------------------------------------------------------------------- */
/*                            Struct implementation                           */
/* -------------------------------------------------------------------------- */
impl sigset_t {
    pub fn add(&mut self, signum: c_int) -> Result<(), io::Error> {
        sigaddset(self, signum)
    }
}

/* -------------------------------------------------------------------------- */
/*                            Safe Function Wrapper                           */
/* -------------------------------------------------------------------------- */
/// return a Termios set with the correct value or an error
pub fn get_terminal_attributes(fd: i32) -> Result<Termios, std::io::Error> {
    let mut termios = Termios {
        c_iflag: 0,
        c_oflag: 0,
        c_cflag: 0,
        c_lflag: 0,
        c_line: 0,
        c_cc: [0; NCCS], // Initialize control characters array
        c_ispeed: 0,
        c_ospeed: 0,
    };

    // Call the raw tcgetattr function
    let result = unsafe { raw::tcgetattr(fd, &mut termios) };

    if result == -1 {
        return Err(std::io::Error::last_os_error());
    }

    Ok(termios)
}

/// restore the old termios setting
pub fn disable_raw_mode(orig_termios: Termios) -> Result<(), io::Error> {
    let fd = io::stdin().as_raw_fd(); // Get the file descriptor for stdin
    let result = unsafe { raw::tcsetattr(fd, TCSANOW, &orig_termios) };

    // Restore original terminal settings
    if result == -1 {
        return Err(std::io::Error::last_os_error());
    }

    Ok(())
}

/// set some attribute on the termios given as argument
pub fn set_terminal_attributes(
    fd: i32,
    optional_actions: i32,
    termios: &Termios,
) -> Result<(), std::io::Error> {
    // Call the raw tcsetattr function
    let result = unsafe { raw::tcsetattr(fd, optional_actions, termios) };

    if result == -1 {
        return Err(std::io::Error::last_os_error());
    }

    Ok(())
}

/// set a new umask returning the old value
pub fn set_umask(new_umask: mode_t) -> mode_t {
    unsafe { raw::umask(new_umask) }
}

pub fn setpwent() {
    unsafe {
        raw::setpwent();
    }
}

pub fn endpwent() {
    unsafe {
        raw::endpwent();
    }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
/// transform a c_string into a String
pub fn ptr_to_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(ptr).to_str().ok().map(|s| s.to_owned()) }
}

/// send a signal to a process
pub fn kill(pid: pid_t, signal: i32) -> std::io::Result<()> {
    let result = unsafe { raw::kill(pid, signal) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

/// return a password struct
pub fn getpwent() -> Option<Passwd> {
    unsafe {
        let pw_ptr = raw::getpwent();
        if pw_ptr.is_null() {
            None
        } else {
            Some(std::ptr::read(pw_ptr))
        }
    }
}

/// return the parent process id, can never fail
pub fn getpid() -> pid_t {
    unsafe { raw::getpid() }
}

/// used to send a signal to a given process
pub fn signal(signum: c_int, handler: sighandler_t) {
    unsafe {
        raw::signal(signum, handler);
    }
}

/// used to add a signal to a given sigset_t
pub fn sigaddset(set: &mut sigset_t, signum: c_int) -> Result<(), std::io::Error> {
    let result = unsafe { raw::sigaddset(set, signum) };
    match result {
        0 => Ok(()),
        _ => Err(std::io::Error::last_os_error()),
    }
}

/// use override the default behavior when receiving a signal
/// # Arguments
/// - `how` define the behavior of the function regarding the other arguments
/// - `set` the set used according to the how argument
/// - ``
pub fn pthread_sigmask(
    how: How,
    set: &sigset_t,
    oldset: Option<&mut sigset_t>,
) -> Result<(), std::io::Error> {
    let result = match oldset {
        Some(oldset) => unsafe { raw::pthread_sigmask(how as c_int, set, oldset) },
        None => unsafe { raw::pthread_sigmask(how as c_int, set, std::ptr::null_mut()) },
    };
    match result {
        0 => Ok(()),
        _ => Err(std::io::Error::last_os_error()),
    }
}
