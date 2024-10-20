use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender};
use rustix::{process, termios};
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, BufReader, Read, Write};
use std::os::fd::{AsRawFd, BorrowedFd, RawFd};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use vte::{Params, Parser, Perform};

pub struct TerminalPlugin;

impl Plugin for TerminalPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (spawn_terminals, update_terminals));
    }
}

pub(crate) enum InternalEvent {
    Print(char),
}

#[derive(Component, Debug)]
pub struct Terminal {
    pub program: OsString,
    pub text_style: TextStyle,
}

#[derive(Debug)]
pub(crate) struct Inner {}

#[derive(Component, Debug)]
pub struct InternalTerminalState {
    pub(crate) child_process: Child,
    pub(crate) inner: Inner,
    pub(crate) reader_handle: JoinHandle<io::Result<()>>,
    pub(crate) reader_receiver: Receiver<InternalEvent>,
    pub(crate) writer_handle: JoinHandle<io::Result<()>>,
    pub(crate) writer_sender: Sender<InternalEvent>,
}

impl InternalTerminalState {
    fn process_events(&mut self, terminal: &Terminal, text: &mut Mut<'_, Text>) {
        for event in self.reader_receiver.try_iter() {
            match event {
                InternalEvent::Print(character) => {
                    if text.sections.is_empty() {
                        text.sections.push(TextSection {
                            style: terminal.text_style.clone(),
                            ..default()
                        });
                    }

                    text.sections[0].value.push(character);
                }
            }
        }
    }
}

fn spawn_terminals(
    mut commands: Commands,
    mut query: Query<(Entity, &Terminal), Without<InternalTerminalState>>,
) {
    for (entity, terminal) in query.iter_mut() {
        match try_spawn(&terminal) {
            Ok(internal_state) => {
                commands.entity(entity).insert(internal_state);
            }
            Err(_error) => {
                commands.entity(entity).remove::<Terminal>();
            }
        }
    }
}

fn try_spawn(terminal: &Terminal) -> io::Result<InternalTerminalState> {
    let mut command = Command::new(&terminal.program);

    let pty = rustix_openpty::openpty(None, None)?;
    let mut writer_control_fd = Arc::new(File::from(pty.controller));
    let user_fd = pty.user;

    command
        .env("COLORTERM", "truecolor")
        .env("TERM", "xterm-256color")
        .stdin(user_fd.try_clone()?)
        .stdout(user_fd.try_clone()?)
        .stderr(user_fd.try_clone()?);

    let user_fd = user_fd.as_raw_fd();

    unsafe {
        command.pre_exec(move || pre_exec(user_fd));
    }

    let (reader_sender, reader_receiver) = crossbeam_channel::unbounded();
    let (writer_sender, writer_receiver) = crossbeam_channel::unbounded();

    let reader_control_fd = Arc::clone(&writer_control_fd);
    let reader_handle = thread::spawn(move || {
        let mut parser = Parser::new();
        let mut performer = Performer::new(reader_sender);

        for result in BufReader::new(reader_control_fd).bytes() {
            match result {
                Ok(byte) => {
                    parser.advance(&mut performer, byte);
                }
                Err(error) => {}
            }
        }

        io::Result::Ok(())
    });

    let writer_handle = thread::spawn(move || {
        let mut buf = [0; 4];

        while let Ok(internal_event) = writer_receiver.recv() {
            match internal_event {
                InternalEvent::Print(character) => {
                    let bytes = character.encode_utf8(&mut buf).as_bytes();

                    writer_control_fd.write_all(bytes)?;
                }
            }
        }

        io::Result::Ok(())
    });

    let child_process = command.spawn()?;
    let inner = Inner {};

    Ok(InternalTerminalState {
        child_process,
        inner,
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

fn update_terminals(
    mut reader: EventReader<KeyboardInput>,
    mut query: Query<(&Terminal, &mut InternalTerminalState, &mut Text)>,
) {
    for (terminal, mut state, mut text) in query.iter_mut() {
        state.process_events(&terminal, &mut text);

        for event in reader.read() {
            if !event.state.is_pressed() {
                continue;
            }

            match &event.logical_key {
                Key::Character(string) => {
                    for character in string.chars() {
                        state.writer_sender.send(InternalEvent::Print(character));
                    }
                }
                Key::Enter => {
                    state.writer_sender.send(InternalEvent::Print('\n'));
                }
                _ => {}
            }
        }
    }
}

pub(crate) struct Performer {
    reader_sender: Sender<InternalEvent>,
}

impl Performer {
    pub fn new(reader_sender: Sender<InternalEvent>) -> Self {
        Self { reader_sender }
    }
}

impl Perform for Performer {
    fn print(&mut self, character: char) {
        if let Err(_error) = self.reader_sender.send(InternalEvent::Print(character)) {
            //
        }
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            0x0A => {
                if let Err(_error) = self.reader_sender.send(InternalEvent::Print('\n')) {
                    //
                }
            }
            _ => {}
        }
    }
}
