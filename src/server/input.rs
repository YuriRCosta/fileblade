use super::*;
use std::io::Read;
use std::os::fd::AsFd;

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

extern "C" fn interrupt(_: libc::c_int) {
    INTERRUPTED.store(true, Ordering::Relaxed);
}

pub(super) fn interrupted() -> bool {
    INTERRUPTED.load(Ordering::Relaxed)
}

pub(super) struct InputSignals {
    term: libc::sigaction,
    int: libc::sigaction,
}

impl InputSignals {
    pub(super) fn install() -> io::Result<Self> {
        INTERRUPTED.store(false, Ordering::Relaxed);
        unsafe {
            let mut action: libc::sigaction = std::mem::zeroed();
            action.sa_sigaction = interrupt as *const () as usize;
            libc::sigemptyset(&mut action.sa_mask);
            let mut previous = Self {
                term: std::mem::zeroed(),
                int: std::mem::zeroed(),
            };
            if libc::sigaction(libc::SIGTERM, &action, &mut previous.term) != 0 {
                return Err(io::Error::last_os_error());
            }
            if libc::sigaction(libc::SIGINT, &action, &mut previous.int) != 0 {
                let error = io::Error::last_os_error();
                libc::sigaction(libc::SIGTERM, &previous.term, std::ptr::null_mut());
                return Err(error);
            }
            Ok(previous)
        }
    }
}

impl Drop for InputSignals {
    fn drop(&mut self) {
        unsafe {
            libc::sigaction(libc::SIGTERM, &self.term, std::ptr::null_mut());
            libc::sigaction(libc::SIGINT, &self.int, std::ptr::null_mut());
        }
    }
}

pub(super) struct InterruptibleStdin;

impl Read for InterruptibleStdin {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let stdin = io::stdin();
        loop {
            if buffer.is_empty() || interrupted() {
                return Ok(0);
            }
            let mut descriptors = [PollFd::new(&stdin, PollFlags::IN)];
            match poll(&mut descriptors, Some(&WATCH_POLL_TIMEOUT)) {
                Ok(0) | Err(Errno::INTR) => continue,
                Ok(_) => {}
                Err(error) => return Err(error.into()),
            }
            match rustix::io::read(stdin.as_fd(), &mut *buffer) {
                Err(Errno::INTR) => continue,
                result => return result.map_err(io::Error::from),
            }
        }
    }
}
