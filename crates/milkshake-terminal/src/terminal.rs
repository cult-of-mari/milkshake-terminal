use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender};
use milkshake_vte::{Vte, VteHandler};
use rustix::process;
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, BufReader, Read, Write};
use std::os::fd::{AsRawFd, BorrowedFd, RawFd};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

pub struct TerminalPlugin;

impl Plugin for TerminalPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (spawn_terminals, update_terminals));
    }
}

pub(crate) enum ReadEvent {
    Print(char),
    Backspace,
}

pub(crate) enum WriteEvent {
    Input(char),
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
    pub(crate) reader_receiver: Receiver<ReadEvent>,
    pub(crate) writer_handle: JoinHandle<io::Result<()>>,
    pub(crate) writer_sender: Sender<WriteEvent>,
}

impl InternalTerminalState {
    fn process_events(&mut self, terminal: &Terminal, text: &mut Mut<'_, Text>) {
        for event in self.reader_receiver.try_iter() {
            match event {
                ReadEvent::Print(character) => {
                    if text.sections.is_empty() {
                        text.sections.push(TextSection {
                            style: terminal.text_style.clone(),
                            ..default()
                        });
                    }

                    text.sections[0].value.push(character);
                }
                ReadEvent::Backspace => {
                    if text.sections.is_empty() {
                        continue;
                    }

                    text.sections[0].value.pop();
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
        match try_spawn(terminal) {
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
        let bytes = BufReader::new(reader_control_fd).bytes();
        let mut vte = Vte::new(Handler { reader_sender });

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
        state.process_events(terminal, &mut text);

        for event in reader.read() {
            if !event.state.is_pressed() {
                continue;
            }

            match &event.logical_key {
                Key::Character(string) => {
                    for character in string.chars() {
                        state.writer_sender.send(WriteEvent::Input(character));
                    }
                }
                Key::Enter => {
                    state.writer_sender.send(WriteEvent::Input('\n'));
                }
                Key::Space => {
                    state.writer_sender.send(WriteEvent::Input(' '));
                }
                Key::Backspace => {
                    state.writer_sender.send(WriteEvent::Input('\x08'));
                }
                _ => {}
            }
        }
    }
}

pub(crate) struct Handler {
    reader_sender: Sender<ReadEvent>,
}

impl VteHandler for Handler {
    fn input(&mut self, character: char) {
        self.reader_sender.send(ReadEvent::Print(character));
    }

    fn backspace(&mut self) {
        self.reader_sender.send(ReadEvent::Backspace);
    }

    fn newline(&mut self) {
        self.reader_sender.send(ReadEvent::Print('\n'));
    }
}
