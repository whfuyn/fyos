use crate::vga_buffer::*;

pub struct Console {
    vga_buffer: VgaBuffer,
    // Cursor
    row: usize,
    col: usize,
}

impl Console {
    pub fn new() -> Self {
        Self {
            vga_buffer: VgaBuffer::new(),
            row: 0,
            col: 0,
        }
    }

    /// See [`put_char`] for details
    pub fn puts<C: Into<VgaChar> + Copy>(&mut self, s: &[C]) {
        for &ch in s {
            self.put_char(ch);
        }
    }

    /// Put a char on the current cursor. Move to the next line if
    /// it hits the right boundary. Shift the whole lines up if
    /// it reaches the bottom line.
    /// Caveat:
    /// 1. We treat '\r' as '\r', '\n' as '\r\n'.
    /// 2. It doesn't flush until `\r` or `\n`.
    pub fn put_char<C: Into<VgaChar> + Copy>(&mut self, ch: C) {
        let ch = ch.into();
        if self.col == VGA_BUFFER_COLUMNS {
            self.next_line();
        }
        match ch.code_point() {
            b'\r' => {
                self.col = 0;
                self.vga_buffer.sync();
                return;
            }
            b'\n' => {
                self.next_line();
                self.vga_buffer.sync();
                return;
            }
            _ => (),
        }
        self.vga_buffer.buffer[self.row][self.col] = ch;
        self.col += 1;
    }

    /// Move the cursor to next line. If we are already at the bottom line,
    /// move all the lines upward.
    fn next_line(&mut self) {
        if self.row + 1 < VGA_BUFFER_ROWS {
            self.row += 1;
            return;
        }
        // Move all the lines up.
        // TODO: add the discarded line to history.
        let buffer = &mut self.vga_buffer.buffer;
        for row in 0..(VGA_BUFFER_ROWS - 1) {
            let (first, rest) = buffer[row..].split_first_mut().unwrap();
            first.copy_from_slice(&rest[0]);
        }
        // Clear the last line.
        buffer.last_mut().unwrap().fill(VgaChar::default());

        // self.row remains unchanged.
        self.col = 0;
    }
}
