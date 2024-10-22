use bevy::log::error;
use bevy::math::U16Vec2;
use compact_str::CompactString;
use crossbeam_channel::Sender;
use milkshake_vte::VteHandler;

#[derive(Debug)]
pub enum ReadEvent {
    Print(char),
    Backspace,
    Newline,

    MoveLeft(u16),
    MoveRight(u16),
    MoveUp(u16),
    MoveDown(u16),

    MoveTo(U16Vec2),

    MoveToLineStart,

    MoveUpToLineStart(u16),
    MoveDownToLineStart(u16),
}

#[derive(Debug)]
pub enum WriteEvent {
    Input(CompactString),
}

pub struct Handler {
    sender: Sender<ReadEvent>,
}

impl Handler {
    pub fn new(sender: Sender<ReadEvent>) -> Self {
        Self { sender }
    }

    fn send(&self, event: ReadEvent) {
        if let Err(error) = self.sender.send(event) {
            error!("{error}");
        }
    }
}

impl VteHandler for Handler {
    fn input(&mut self, character: char) {
        let event = match character {
            ' ' => ReadEvent::MoveRight(1),
            _ => ReadEvent::Print(character),
        };

        self.send(event);
    }

    fn backspace(&mut self) {
        self.send(ReadEvent::Backspace);
    }

    fn newline(&mut self) {
        self.send(ReadEvent::Newline);
    }

    fn move_up(&mut self, rows: u16) {
        self.send(ReadEvent::MoveUp(rows));
    }

    fn move_down(&mut self, rows: u16) {
        self.send(ReadEvent::MoveDown(rows));
    }

    fn move_left(&mut self, cols: u16) {
        self.send(ReadEvent::MoveLeft(cols));
    }

    fn move_right(&mut self, cols: u16) {
        self.send(ReadEvent::MoveRight(cols));
    }

    fn move_to(&mut self, row: u16, col: u16) {
        self.send(ReadEvent::MoveTo(U16Vec2::new(col, row)));
    }

    fn move_to_line_start(&mut self) {
        self.send(ReadEvent::MoveToLineStart);
    }

    fn move_up_to_line_start(&mut self, rows: u16) {
        self.send(ReadEvent::MoveUpToLineStart(rows));
    }

    fn move_down_to_line_start(&mut self, rows: u16) {
        self.send(ReadEvent::MoveDownToLineStart(rows));
    }
}
