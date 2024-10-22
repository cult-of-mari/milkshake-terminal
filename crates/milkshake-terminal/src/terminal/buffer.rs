use bevy::math::U16Vec2;
use bevy::prelude::*;

pub const PLACEHOLDER: [Entity; 2] = [Entity::PLACEHOLDER; 2];

/// A grid-based terminal buffer.
#[derive(Debug, Default)]
pub struct Buffer {
    cells: Vec<[Entity; 2]>,
    pub cursor_position: U16Vec2,
    size: U16Vec2,
}

impl Buffer {
    /// Create a new empty buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Resize the buffer to the specified size.
    pub fn resize(&mut self, size: U16Vec2) {
        let area = size.element_product() as usize;

        self.cells.resize(area, PLACEHOLDER);
        self.size = size;
    }

    /// Returns the cursor position.
    pub fn cursor_position(&self) -> U16Vec2 {
        self.cursor_position
    }

    /// Returns the size of the buffer.
    pub fn size(&mut self) -> U16Vec2 {
        self.size
    }

    /// Returns the offset of the buffer.
    fn offset_of(&self, position: U16Vec2) -> usize {
        ((position.y * self.size.x) + position.x) as usize
    }

    /// Returns a reference to the specified cell.
    pub fn cell(&self, position: U16Vec2) -> Option<&[Entity; 2]> {
        let offset = self.offset_of(position);

        self.cells.get(offset)
    }

    /// Returns a reference to the specified cell.
    pub fn cell_mut(&mut self, position: U16Vec2) -> Option<&mut [Entity; 2]> {
        let offset = self.offset_of(position);

        self.cells.get_mut(offset)
    }

    pub fn move_to_line_start(&mut self) {
        self.cursor_position.x = 0;
    }

    pub fn move_up(&mut self, rows: u16) {
        self.cursor_position.y = self.cursor_position.y.saturating_sub(rows);
    }

    pub fn move_down(&mut self, rows: u16) {
        self.cursor_position.y = self.cursor_position.y.saturating_add(rows).min(self.size.x);
    }

    pub fn move_up_to_line_start(&mut self, rows: u16) {
        self.move_up(rows);
        self.move_to_line_start();
    }

    pub fn move_down_to_line_start(&mut self, rows: u16) {
        self.move_down(rows);
        self.move_to_line_start();
    }

    pub fn move_left(&mut self, columns: u16) {
        self.cursor_position.x = self.cursor_position.x.saturating_sub(columns);
    }

    pub fn move_right(&mut self, columns: u16) {
        self.cursor_position.x = self
            .cursor_position
            .x
            .saturating_add(columns)
            .min(self.size.x);
    }
}
