use self::args::Args;
use self::performer::Performer;

mod args;
mod performer;

pub trait VteHandler {
    fn input(&mut self, character: char) {
        let _character = character;
    }

    fn backspace(&mut self) {}
    fn newline(&mut self) {}

    fn move_up(&mut self, rows: u16) {}
    fn move_down(&mut self, rows: u16) {}
    fn move_left(&mut self, cols: u16) {}
    fn move_right(&mut self, cols: u16) {}

    fn move_to(&mut self, row: u16, col: u16) {}
}

pub struct Vte<T: VteHandler> {
    performer: Performer<T>,
    parser: vte::Parser<1024>,
}

impl<T: VteHandler> Vte<T> {
    pub fn new(state: T) -> Self {
        let performer = Performer::new(state);
        let parser = vte::Parser::new();

        Self { performer, parser }
    }

    pub fn process(&mut self, byte: u8) {
        self.parser.advance(&mut self.performer, byte);
    }
}
