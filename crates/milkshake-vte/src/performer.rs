use super::{Args, VteHandler};

pub struct Performer<T: VteHandler> {
    state: T,
}

impl<T: VteHandler> Performer<T> {
    pub fn new(state: T) -> Self {
        Self { state }
    }
}

impl<T: VteHandler> vte::Perform for Performer<T> {
    fn print(&mut self, character: char) {
        self.state.input(character);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\x08' => self.state.backspace(),
            b'\r' => self.state.move_to_line_start(),
            b'\n' => self.state.newline(),
            _ => {}
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let mut args = Args::new(params.iter());

        match (action, intermediates) {
            ('A', []) => self.state.move_up(args.one_based()),
            ('B', []) => self.state.move_down(args.one_based()),
            ('C', []) => self.state.move_right(args.one_based()),
            ('D', []) => self.state.move_left(args.one_based()),
            ('H', []) | ('f', []) => self.state.move_to(args.one_based(), args.one_based()),
            ('E', []) => self.state.move_up_to_line_start(args.one_based()),
            ('F', []) => self.state.move_down_to_line_start(args.one_based()),
            _ => {}
        }
    }
}
