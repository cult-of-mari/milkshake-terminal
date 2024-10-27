use self::vte::{Vte, VteEvent};
use bevy::color::palettes::css;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use compact_str::CompactString;
use crossbeam_channel::{Receiver, Sender};
use pseudo_terminal::PseudoTerminal;
use std::io::{Read, Write};
use std::process::Command;
use std::{io, mem, thread};

mod convert;
mod pseudo_terminal;
mod vte;

#[derive(Clone, Copy, Component, Debug, Default, Reflect)]
#[reflect(Component, Debug, Default)]
#[require(Node)]
pub struct Terminal;

#[derive(Clone, Copy, Component, Debug, Default, Reflect)]
#[reflect(Component, Debug, Default)]
#[require(Node, Text)]
pub struct TerminalCell;

#[derive(Component, Debug)]
#[require(Terminal)]
pub struct TerminalCommand(pub Command);

#[derive(Clone, Debug, Reflect, Component)]
pub struct TerminalFonts {
    pub regular: Handle<Font>,
    pub regular_italic: Handle<Font>,
    pub bold: Handle<Font>,
    pub bold_italic: Handle<Font>,
}

#[derive(Clone, Copy, Debug, Default)]
struct TerminalStyle {
    foreground: Color,
    background: Color,
    bold: bool,
    italic: bool,
}

#[derive(Clone, Copy, Debug, Default)]
struct TerminalState {
    cursor_position: UVec2,
    style: TerminalStyle,
}

impl TerminalState {
    pub fn cursor_position(&self) -> UVec2 {
        self.cursor_position
    }

    pub fn cursor_offset(&self, width: u32) -> usize {
        ((self.cursor_position.y * width) + self.cursor_position.x) as usize
    }

    pub fn move_up(&mut self, rows: u32) {
        debug_assert!(rows >= 1);

        self.cursor_position.y = self.cursor_position.y.saturating_sub(rows);
    }

    pub fn move_down(&mut self, rows: u32) {
        debug_assert!(rows >= 1);

        self.cursor_position.y = self.cursor_position.y.saturating_add(rows);
    }

    pub fn move_left(&mut self, columns: u32) {
        debug_assert!(columns >= 1);

        self.cursor_position.x = self.cursor_position.x.saturating_sub(columns);
    }

    pub fn move_right(&mut self, columns: u32) {
        debug_assert!(columns >= 1);

        self.cursor_position.x = self.cursor_position.x.saturating_add(columns);
    }

    pub fn goto(&mut self, position: UVec2) {
        self.cursor_position = position;
    }

    pub fn goto_x(&mut self, x: u32) {
        self.cursor_position.x = x;
    }

    pub fn goto_y(&mut self, y: u32) {
        self.cursor_position.y = y;
    }

    pub fn line_up(&mut self, rows: u32) {
        self.move_up(rows);
        self.goto_x(0);
    }

    pub fn line_down(&mut self, rows: u32) {
        self.move_down(rows);
        self.goto_x(0);
    }

    pub fn style(&self) -> TerminalStyle {
        self.style
    }

    pub fn set_bold(&mut self) {
        self.style.bold = true;
    }

    pub fn set_italic(&mut self) {
        self.style.italic = true;
    }

    pub fn reset(&mut self) {
        self.style = default();
        self.style.foreground = Color::WHITE;
        self.style.background = Color::NONE;
    }
}

#[derive(Debug, Component)]
pub struct InternalTerminalState {
    cells: Vec<Entity>,
    pseudo_terminal: PseudoTerminal,
    writer: Sender<CompactString>,
    reader: Receiver<VteEvent>,
    state: TerminalState,
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (setup_terminal, update))
        .run();
}

fn setup(asset_server: Res<AssetServer>, mut commands: Commands) {
    commands.spawn(Camera3d::default());

    commands.spawn((
        Node {
            display: Display::Grid,
            grid_template_columns: RepeatedGridTrack::px(400, 10.0),
            grid_template_rows: RepeatedGridTrack::px(100, 18.0),
            height: Val::Percent(1000.0),
            width: Val::Percent(1000.0),
            ..default()
        },
        Terminal,
        TerminalCommand(Command::new("fish")),
        TerminalFonts {
            regular: asset_server.load("fonts/RobotoMono-SemiBold.ttf"),
            regular_italic: asset_server.load("fonts/RobotoMono-SemiBoldItalic.ttf"),
            bold: asset_server.load("fonts/RobotoMono-Bold.ttf"),
            bold_italic: asset_server.load("fonts/RobotoMono-BoldItalic.ttf"),
        },
    ));
}

struct Handler(Sender<VteEvent>);

impl vte::VteHandler for Handler {
    fn vte_event(&mut self, event: VteEvent) {
        self.0.send(event).unwrap();
    }
}

pub fn setup_terminal(
    mut commands: Commands,
    mut query: Query<(Entity, &mut TerminalCommand), Without<InternalTerminalState>>,
) {
    for (entity, mut command) in query.iter_mut() {
        let mut pseudo_terminal = PseudoTerminal::new(UVec2::new(400, 100)).unwrap();

        pseudo_terminal.configure_command(&mut command.0).unwrap();

        let reader = {
            let (sender, receiver) = crossbeam_channel::unbounded::<VteEvent>();
            let mut control = pseudo_terminal.control.clone();

            thread::spawn(move || {
                let mut vte = Vte::new(Handler(sender));
                let mut buf = [0; 1024];

                while let Ok(amount) = control.read(&mut buf) {
                    vte.process(&buf[..amount]);
                }
            });

            receiver
        };

        let writer = {
            let (sender, receiver) = crossbeam_channel::unbounded::<CompactString>();
            let mut control = pseudo_terminal.control.clone();

            thread::spawn(move || {
                for text in receiver.iter() {
                    control.write_all(text.as_bytes())?;
                }

                io::Result::Ok(())
            });

            sender
        };

        command.0.spawn().unwrap();

        let internal_terminal_state = InternalTerminalState {
            cells: vec![Entity::PLACEHOLDER; 400 * 100],
            pseudo_terminal,
            reader,
            writer,
            state: default(),
        };

        commands.entity(entity).insert(internal_terminal_state);
    }
}

static TABLE: [Srgba; 8] = [
    css::BLACK,
    css::RED,
    css::GREEN,
    css::YELLOW,
    css::BLUE,
    css::MAGENTA,
    css::LIGHT_CYAN,
    css::WHITE,
];

fn update(
    mut commands: Commands,
    mut keyboard_input: EventReader<KeyboardInput>,
    mut query: Query<(Entity, &mut InternalTerminalState)>,
    terminal_fonts: Query<&TerminalFonts>,
) {
    for (entity, mut state) in query.iter_mut() {
        let InternalTerminalState {
            cells,
            reader,
            writer,
            state,
            ..
        } = &mut *state;

        for event in reader.try_iter() {
            match event {
                VteEvent::Echo(character) => {
                    let Some(cell_entity) = cells.get_mut(state.cursor_offset(400)) else {
                        continue;
                    };

                    commands.entity(entity).with_children(|builder| {
                        let terminal_fonts = terminal_fonts.get(builder.parent_entity()).unwrap();

                        if *cell_entity == Entity::PLACEHOLDER {
                            *cell_entity = new_cell(state, terminal_fonts, builder, character);
                        }
                    });

                    state.move_right(1);
                }
                VteEvent::Backspace => {
                    state.move_left(1);

                    let Some(entity) = cells.get_mut(state.cursor_offset(400)) else {
                        continue;
                    };

                    let entity = mem::replace(entity, Entity::PLACEHOLDER);

                    if entity != Entity::PLACEHOLDER {
                        commands.entity(entity).despawn_recursive();
                    }
                }

                VteEvent::Goto(new_position) => state.goto(new_position),
                VteEvent::GotoX(x) => state.goto_x(x),
                VteEvent::GotoY(y) => state.goto_y(y),

                VteEvent::LineUp(rows) => state.line_up(rows),
                VteEvent::LineDown(rows) => state.line_down(rows),

                VteEvent::MoveUp(rows) => state.move_up(rows),
                VteEvent::MoveDown(rows) => state.move_down(rows),
                VteEvent::MoveLeft(columns) => state.move_left(columns),
                VteEvent::MoveRight(columns) => state.move_right(columns),

                VteEvent::Reset => state.reset(),
                VteEvent::Bold => state.set_bold(),
                VteEvent::Italic => state.set_italic(),
                VteEvent::Foreground(color) => {
                    state.style.foreground = TABLE[color as usize].into();
                }
                VteEvent::Background(color) => {
                    state.style.background = TABLE[color as usize].into();
                }
                _ => {}
            }
        }

        for event in keyboard_input.read() {
            if !event.state.is_pressed() {
                continue;
            }

            if let Some(string) = convert::convert_key(&event.logical_key) {
                writer.send(string).unwrap();
            }
        }
    }
}

fn new_cell(
    terminal_state: &mut TerminalState,
    terminal_fonts: &TerminalFonts,
    builder: &mut ChildBuilder<'_>,
    character: char,
) -> Entity {
    let [grid_column, grid_row] = (terminal_state.cursor_position() + UVec2::ONE)
        .to_array()
        .map(|axis| GridPlacement::start(axis as i16));

    let font = match (terminal_state.style.bold, terminal_state.style.italic) {
        (true, true) => &terminal_fonts.bold_italic,
        (true, false) => &terminal_fonts.bold,
        (false, true) => &terminal_fonts.regular_italic,
        (false, false) => &terminal_fonts.regular,
    };

    builder
        .spawn((
            BackgroundColor(terminal_state.style.background),
            Node {
                grid_column,
                grid_row,
                ..default()
            },
            TerminalCell,
            Text::new(character),
            TextColor(terminal_state.style.foreground),
            TextFont {
                font: font.clone(),
                font_size: 14.0,
                ..default()
            },
        ))
        .id()
}
