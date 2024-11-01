use bevy::math::UVec2;
use rustix::process;
use rustix::termios::Winsize;
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, BorrowedFd, OwnedFd, RawFd};
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::sync::Arc;

#[derive(Debug)]
pub struct PseudoTerminal {
    pub control: Arc<File>,
    pub user: OwnedFd,
}

impl PseudoTerminal {
    pub fn new(size: UVec2) -> io::Result<PseudoTerminal> {
        let size = Winsize {
            ws_col: size.x as u16,
            ws_row: size.y as u16,
            ws_xpixel: (size.x * 10) as u16,
            ws_ypixel: (size.y * 18) as u16,
        };

        let pty = rustix_openpty::openpty(None, Some(&size))?;
        let control = Arc::new(File::from(pty.controller));
        let user = pty.user;

        Ok(PseudoTerminal { control, user })
    }

    pub fn configure_command(&mut self, command: &mut Command) -> io::Result<()> {
        let Self { user, .. } = self;

        command
            .env("TERM", "xterm-256color")
            .env("COLORTERM", "truecolor")
            .stdin(user.try_clone()?)
            .stdout(user.try_clone()?)
            .stderr(user.try_clone()?);

        unsafe {
            let user = user.as_raw_fd();

            command.pre_exec(move || set_controlling_terminal(user));
        }

        Ok(())
    }
}

fn set_controlling_terminal(user: RawFd) -> io::Result<()> {
    process::setsid()?;
    process::ioctl_tiocsctty(unsafe { BorrowedFd::borrow_raw(user) })?;

    for fd in 3..1000 {
        unsafe {
            libc::close(fd);
        }
    }

    Ok(())
}
