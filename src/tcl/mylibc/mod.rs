use std::ffi::{c_char, c_int, CStr};

mod raw;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Termios {
    pub c_iflag: tcflag_t,  // Input mode flags
    pub c_oflag: tcflag_t,  // Output mode flags
    pub c_cflag: tcflag_t,  // Control mode flags
    pub c_lflag: tcflag_t,  // Local mode flags
    pub c_line: u8,         // Line discipline
    pub c_cc: [cc_t; NCCS], // Control characters
    pub c_ispeed: speed_t,  // Input speed
    pub c_ospeed: speed_t,  // Output speed
}

// Define types for compatibility with C
pub type tcflag_t = u32; // or u16 depending on your platform
pub type cc_t = u8; // Control character type
pub type speed_t = u32; // Speed type (usually an unsigned integer)

pub type mode_t = u32; // or u16 on some systems
pub type uid_t = u32; // or u16 on some systems
pub type gid_t = u32; // or u16 on some systems
pub type pid_t = i32;

pub const NCCS: usize = 32; // Number of control characters

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
pub const ECHO: i32 = 0o00000100; // Enable echoing of input characters
pub const ICANON: i32 = 0o00000002; // Canonical mode (line buffering)
pub const TCSANOW: i32 = 0; // Change attributes immediately

#[repr(C)]
pub struct passwd {
    pub pw_name: *mut c_char,
    pw_passwd: *mut c_char,
    pub pw_uid: uid_t,
    pub pw_gid: gid_t,
    pw_gecos: *mut c_char,
    pw_dir: *mut c_char,
    pw_shell: *mut c_char,
}

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

use std::io;
use std::os::unix::io::AsRawFd;

pub fn disable_raw_mode(orig_termios: Termios) {
    let fd = io::stdin().as_raw_fd(); // Get the file descriptor for stdin
    unsafe {
        // Restore original terminal settings
        if raw::tcsetattr(fd, TCSANOW, &orig_termios) == -1 {
            eprintln!(
                "Error restoring terminal settings: {}",
                std::io::Error::last_os_error()
            );
        }
    }
}

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

pub fn ptr_to_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(ptr).to_str().ok().map(|s| s.to_owned()) }
}

pub fn kill(pid: pid_t, signal: i32) -> std::io::Result<()> {
    let result = unsafe { raw::kill(pid, signal) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

pub fn getpwent() -> Option<passwd> {
    unsafe {
        let pw_ptr = raw::getpwent();
        if pw_ptr.is_null() {
            None
        } else {
            Some(std::ptr::read(pw_ptr))
        }
    }
}
