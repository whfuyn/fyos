use crate::sync::SpinLock;
use crate::vga_buffer::*;
use core::fmt::Write;

pub static KONSOLE: SpinLock<Konsole> = SpinLock::new(Konsole::new());

// #[macro_export]
// macro_rules! print {
//     ($($arg:tt)*) => {{
//         use ::core::fmt::Write;
//         let mut konsole = $crate::konsole::KONSOLE.lock();
//         write!(&mut konsole, "{}", core::format_args!($($arg)*))
//     }};
// }

// #[macro_export]
// macro_rules! println {
//     () => {
//         print("\n")
//     };
//     ($($arg:tt)*) => {{
//         use ::core::fmt::Write;
//         let mut konsole = $crate::konsole::KONSOLE.lock();
//         write!(&mut konsole, "{}", core::format_args_nl!($($arg)*))
//     }};
// }

pub struct Konsole {
    vga_buffer: VgaBuffer,
    // Cursor
    row: usize,
    col: usize,
}

impl Konsole {
    pub const fn new() -> Self {
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
                self.vga_buffer.flush();
                return;
            }
            b'\n' => {
                self.next_line();
                self.vga_buffer.flush();
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
            self.col = 0;
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

impl Write for Konsole {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        if !s.is_ascii() {
            return Err(core::fmt::Error);
        }
        self.puts(s.as_bytes());
        Ok(())
    }
}
