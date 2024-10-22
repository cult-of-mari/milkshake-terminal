use super::buffer::Buffer;
use super::handler::{Handler, ReadEvent, WriteEvent};
use super::Terminal;
use bevy::math::U16Vec2;
use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender};
use milkshake_vte::Vte;
use rustix::process;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::os::fd::{AsRawFd, BorrowedFd, RawFd};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::{io, thread};

#[derive(Component, Debug)]
pub struct InternalTerminalState {
    pub(super) buffer: Buffer,
    child_process: Child,
    reader_handle: JoinHandle<io::Result<()>>,
    pub(super) reader_receiver: Receiver<ReadEvent>,
    writer_handle: JoinHandle<io::Result<()>>,
    writer_sender: Sender<WriteEvent>,
}

impl InternalTerminalState {
    pub(super) fn new(terminal: &Terminal) -> io::Result<Self> {
        let mut command = Command::new(&terminal.program);

        let pty = rustix_openpty::openpty(None, None)?;
        let mut control = Arc::new(File::from(pty.controller));
        let user = pty.user;

        command
            .env("COLORTERM", "truecolor")
            .env("TERM", "xterm-256color")
            .stdin(user.try_clone()?)
            .stdout(user.try_clone()?)
            .stderr(user.try_clone()?);

        let user_fd = user.as_raw_fd();

        unsafe {
            command.pre_exec(move || Self::pre_exec(user_fd));
        }

        let (reader_sender, reader_receiver) = crossbeam_channel::unbounded();
        let (writer_sender, writer_receiver) = crossbeam_channel::unbounded();

        let bytes = BufReader::new(control.clone()).bytes();
        let mut vte = Vte::new(Handler::new(reader_sender));

        let reader_handle = thread::spawn(move || {
            for byte in bytes {
                let byte = byte.inspect_err(|error| error!("read from control fd: {error}"))?;

                vte.process(byte);
            }

            io::Result::Ok(())
        });

        let writer_handle = thread::spawn(move || {
            let mut buf = [0; 4];

            while let Ok(internal_event) = writer_receiver.recv() {
                match internal_event {
                    WriteEvent::Input(character) => {
                        let bytes = character.encode_utf8(&mut buf).as_bytes();

                        control.write_all(bytes)?;
                    }
                }
            }

            io::Result::Ok(())
        });

        let child_process = command.spawn()?;
        let mut buffer = Buffer::new();

        buffer.resize(U16Vec2::new(105, 65));

        Ok(Self {
            buffer,
            child_process,
            reader_handle,
            reader_receiver,
            writer_handle,
            writer_sender,
        })
    }

    fn pre_exec(user_fd: RawFd) -> io::Result<()> {
        let user_fd = unsafe { BorrowedFd::borrow_raw(user_fd) };

        process::setsid()?;
        process::ioctl_tiocsctty(user_fd)?;

        // FIXME: this is here to close fds from graphics drivers etc,
        // as no one sets CLOEXEC in 2024...
        (3..=1000).for_each(|fd| unsafe {
            libc::close(fd);
        });

        Ok(())
    }

    pub(super) fn send(&self, write_event: WriteEvent) {
        if let Err(error) = self.writer_sender.send(write_event) {
            error!("{error}");
        }
    }
}
