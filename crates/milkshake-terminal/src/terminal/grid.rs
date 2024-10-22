use bevy::math::U16Vec2;
use bevy::prelude::*;

/// A grid terminal buffer.
#[derive(Debug, Default)]
pub struct Grid {
    // All cells.
    cells: Vec<Entity>,
    // Size.
    size: U16Vec2,
    // Current position.
    cursor_position: U16Vec2,
    // For save/restore.
    last_cursor_position: U16Vec2,
}

impl Grid {
    /// Create a new empty grid.
    pub fn new() -> Self {
        Self::default()
    }

    /// Resize the grid to the specified size.
    pub fn resize(&mut self, size: U16Vec2) {
        let area = size.element_product() as usize;

        self.cells.resize(area, Entity::PLACEHOLDER);
        self.size = size;
    }

    /// Returns the cursor position.
    pub fn cursor_position(&self) -> U16Vec2 {
        self.cursor_position
    }

    /// Returns the size of the grid.
    pub fn size(&mut self) -> U16Vec2 {
        self.size
    }

    /// Returns the offset of the grid.
    fn offset_of(&self, position: U16Vec2) -> usize {
        ((position.y * self.size.x) + position.x) as usize
    }

    /// Returns a reference to the specified cell.
    pub fn cell(&self, position: U16Vec2) -> Option<&Entity> {
        let offset = self.offset_of(position);

        self.cells.get(offset)
    }

    /// Returns a mutable reference to the specified cell.
    pub fn cell_mut(&mut self, position: U16Vec2) -> Option<&mut Entity> {
        let offset = self.offset_of(position);

        self.cells.get_mut(offset)
    }

    /// Moves the cursor to the beginning of the current line.
    pub fn move_to_line_start(&mut self) {
        self.cursor_position.x = 0;
    }

    /// Moves the cursor up by the specified number of rows.
    pub fn move_up(&mut self, rows: u16) {
        self.cursor_position.y = self.cursor_position.y.saturating_sub(rows);
    }

    /// Moves the cursor down by the specified number of rows.
    pub fn move_down(&mut self, rows: u16) {
        self.cursor_position.y = self.cursor_position.y.saturating_add(rows).min(self.size.x);
    }

    /// Moves the cursor up by the specified number of rows and then to the beginning of the line.
    pub fn move_up_to_line_start(&mut self, rows: u16) {
        self.move_up(rows);
        self.move_to_line_start();
    }

    /// Moves the cursor down by the specified number of rows and then to the beginning of the line.
    pub fn move_down_to_line_start(&mut self, rows: u16) {
        self.move_down(rows);
        self.move_to_line_start();
    }

    /// Moves the cursor left by the specified number of columns.
    pub fn move_left(&mut self, columns: u16) {
        self.cursor_position.x = self.cursor_position.x.saturating_sub(columns);
    }

    /// Moves the cursor right by the specified number of columns.
    pub fn move_right(&mut self, columns: u16) {
        self.cursor_position.x = self
            .cursor_position
            .x
            .saturating_add(columns)
            .min(self.size.x);
    }

    pub fn move_to(&mut self, position: U16Vec2) {
        self.cursor_position = position.clamp(U16Vec2::ZERO, self.size);
    }
}
