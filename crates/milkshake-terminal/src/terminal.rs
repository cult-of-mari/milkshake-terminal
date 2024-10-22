use self::handler::ReadEvent;
use self::state::InternalTerminalState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::math::U16Vec2;
use bevy::prelude::*;
use std::ffi::OsString;
use std::mem;

mod grid;
mod handler;
mod state;

pub struct TerminalPlugin;

impl Plugin for TerminalPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (spawn_terminals, update_terminals));
    }
}

#[derive(Component, Debug)]
pub struct Terminal {
    pub program: OsString,
    pub text_style: TextStyle,
    pub size: U16Vec2,
}

pub fn spawn_terminals(
    mut commands: Commands,
    mut query: Query<(Entity, &Terminal, &mut Style), Without<InternalTerminalState>>,
) {
    for (entity, terminal, mut style) in query.iter_mut() {
        style.grid_template_columns = RepeatedGridTrack::flex(terminal.size.x, 1.0);
        style.grid_template_rows = RepeatedGridTrack::flex(terminal.size.y, 1.0);

        match InternalTerminalState::new(terminal) {
            Ok(internal_state) => {
                commands.entity(entity).insert(internal_state);
            }
            Err(_error) => {
                commands.entity(entity).remove::<Terminal>();
            }
        }
    }
}
pub fn update_terminals(
    mut commands: Commands,
    mut reader: EventReader<KeyboardInput>,
    mut terminal_query: Query<(Entity, &Terminal, &mut InternalTerminalState)>,
    mut children_query: Query<&Children>,
    mut text_query: Query<&mut Text>,
) {
    for (terminal_entity, terminal, mut state) in terminal_query.iter_mut() {
        let InternalTerminalState {
            ref mut grid,
            ref mut reader_receiver,
            ..
        } = state.bypass_change_detection();

        for event in reader_receiver.try_iter() {
            match event {
                ReadEvent::Print(character) => {
                    let cursor_position = grid.cursor_position();

                    let Some(cell_entity) = grid.cell_mut(cursor_position) else {
                        return;
                    };

                    if *cell_entity == Entity::PLACEHOLDER {
                        new_cell(
                            &mut commands,
                            terminal,
                            terminal_entity,
                            cursor_position,
                            cell_entity,
                            character,
                        );
                    } else {
                        set_cell(
                            &mut children_query,
                            &mut text_query,
                            *cell_entity,
                            character,
                        );
                    }

                    grid.move_right(1);
                }
                ReadEvent::Newline => grid.move_down_to_line_start(1),
                ReadEvent::MoveToLineStart => grid.move_to_line_start(),
                ReadEvent::Backspace => {
                    let cursor_position = grid.cursor_position();

                    let Some(node_entity) = grid.cell_mut(cursor_position) else {
                        continue;
                    };

                    let node_entity = mem::replace(node_entity, Entity::PLACEHOLDER);

                    if node_entity != Entity::PLACEHOLDER {
                        commands.entity(node_entity).despawn_recursive();
                    }

                    grid.move_left(1);
                }
                ReadEvent::MoveUp(rows) => grid.move_up(rows),
                ReadEvent::MoveDown(rows) => grid.move_down(rows),
                ReadEvent::MoveLeft(columns) => grid.move_left(columns),
                ReadEvent::MoveRight(columns) => grid.move_right(columns),
                ReadEvent::MoveTo(position) => grid.move_to(position),
                ReadEvent::MoveUpToLineStart(rows) => grid.move_up_to_line_start(rows),
                ReadEvent::MoveDownToLineStart(rows) => grid.move_down_to_line_start(rows),
                //_ => {}
            }
        }

        // FIXME: Only send events for focused terminals.
        for event in reader.read() {
            if !event.state.is_pressed() {
                continue;
            }

            match &event.logical_key {
                Key::Character(string) => state.input(string.as_str()),
                Key::Enter => state.input("\n"),
                Key::Space => state.input(" "),
                Key::Backspace => state.input("\x08"),
                Key::Tab => state.input("\t"),
                Key::ArrowUp => state.input("\x1bOA"),
                Key::ArrowDown => state.input("\x1bOB"),
                Key::ArrowLeft => state.input("\x1bOD"),
                Key::ArrowRight => state.input("\x1bOC"),
                _ => {}
            }
        }
    }
}

fn new_cell(
    commands: &mut Commands,
    terminal: &Terminal,
    terminal_entity: Entity,
    cursor_position: U16Vec2,
    cell_entity: &mut Entity,
    character: char,
) {
    let [grid_column, grid_row] =
        <[_; 2]>::from(cursor_position).map(|value| GridPlacement::start((value + 1) as i16));

    commands.entity(terminal_entity).with_children(|builder| {
        let style = Style {
            display: Display::Grid,
            grid_column,
            grid_row,
            ..default()
        };

        let text = Text::from_section(character, terminal.text_style.clone());

        *cell_entity = builder
            .spawn(NodeBundle { style, ..default() })
            .with_children(|builder| {
                *cell_entity = builder.spawn(TextBundle { text, ..default() }).id();
            })
            .id();
    });
}

fn set_cell(
    children_query: &mut Query<&Children>,
    text_query: &mut Query<&mut Text>,
    cell_entity: Entity,
    character: char,
) {
    let mut text = children_query
        .get(cell_entity)
        .ok()
        .and_then(|children| children.first().copied())
        .and_then(|text_entity| text_query.get_mut(text_entity).ok());

    let Some(section) = text.as_mut().and_then(|text| text.sections.get_mut(0)) else {
        return;
    };

    section.value = character.into();
}
