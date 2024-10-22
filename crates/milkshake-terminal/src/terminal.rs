use self::handler::{ReadEvent, WriteEvent};
use self::state::InternalTerminalState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::math::U16Vec2;
use bevy::prelude::*;
use std::ffi::OsString;
use std::mem;

mod buffer;
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
}

pub fn spawn_terminals(
    mut commands: Commands,
    mut query: Query<(Entity, &Terminal), Without<InternalTerminalState>>,
) {
    for (entity, terminal) in query.iter_mut() {
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
    mut text_query: Query<&mut Text>,
) {
    for (entity, terminal, mut state) in terminal_query.iter_mut() {
        let InternalTerminalState {
            ref mut buffer,
            ref mut reader_receiver,
            ..
        } = *state;

        for event in reader_receiver.try_iter() {
            match event {
                ReadEvent::Print(' ') => buffer.move_right(1),
                ReadEvent::Print(character) => {
                    let cursor_position = buffer.cursor_position();

                    let Some([ref mut node_entity, ref mut text_entity]) =
                        buffer.cell_mut(cursor_position)
                    else {
                        continue;
                    };

                    if *node_entity == Entity::PLACEHOLDER {
                        commands.entity(entity).with_children(|builder| {
                            let [grid_column, grid_row] = to_grid_placement(cursor_position);

                            let style = Style {
                                display: Display::Grid,
                                grid_column,
                                grid_row,
                                ..default()
                            };

                            let text = Text::from_section(character, terminal.text_style.clone());

                            *node_entity = builder
                                .spawn(NodeBundle { style, ..default() })
                                .with_children(|builder| {
                                    *text_entity =
                                        builder.spawn(TextBundle { text, ..default() }).id();
                                })
                                .id();
                        });
                    } else if let Ok(mut text) = text_query.get_mut(*text_entity) {
                        text.sections[0].value = character.into();
                    }

                    buffer.move_right(1);
                }
                ReadEvent::Newline => buffer.move_down_to_line_start(1),
                ReadEvent::MoveToLineStart => buffer.move_to_line_start(),
                ReadEvent::Backspace => {
                    let cursor_position = buffer.cursor_position();
                    let Some(cell) = buffer.cell_mut(cursor_position) else {
                        continue;
                    };

                    if cell[0] != Entity::PLACEHOLDER {
                        let [node_entity, _text_entity] = mem::replace(cell, buffer::PLACEHOLDER);

                        commands.entity(node_entity).despawn_recursive();
                    }

                    buffer.move_left(1);
                }
                ReadEvent::MoveUp(rows) => buffer.move_up(rows),
                ReadEvent::MoveDown(rows) => buffer.move_down(rows),
                ReadEvent::MoveLeft(columns) => buffer.move_left(columns),
                ReadEvent::MoveRight(columns) => buffer.move_right(columns),
                ReadEvent::MoveTo(position) => {
                    buffer.cursor_position = position.clamp(U16Vec2::ONE, buffer.size());
                }
                ReadEvent::MoveUpToLineStart(rows) => buffer.move_up_to_line_start(rows),
                ReadEvent::MoveDownToLineStart(rows) => buffer.move_down_to_line_start(rows),
                //_ => {}
            }
        }

        // FIXME: Only send events for focused terminals.
        for event in reader.read() {
            if !event.state.is_pressed() {
                continue;
            }

            match &event.logical_key {
                Key::Character(string) => {
                    for character in string.chars() {
                        state.send(WriteEvent::Input(character));
                    }
                }
                Key::Enter => state.send(WriteEvent::Input('\n')),
                Key::Space => state.send(WriteEvent::Input(' ')),
                Key::Backspace => state.send(WriteEvent::Input('\x08')),
                _ => {}
            }
        }
    }
}

fn to_grid_placement(position: U16Vec2) -> [GridPlacement; 2] {
    <[_; 2]>::from(position).map(|value| GridPlacement::start((value + 1) as i16))
}
