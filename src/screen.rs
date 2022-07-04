use crate::lazy_static;
use crate::spinlock::SpinLock;

const BRIGHT_BIT: u8 = 1 << 3;
#[allow(dead_code)]
const BLINK_BIT: u8 = 1 << 7;

const VGA_BUFFER_ROWS: usize = 25;
const VGA_BUFFER_COLUMNS: usize = 80;
#[allow(dead_code)]
const VGA_BUFFER_SIZE: usize = VGA_BUFFER_COLUMNS * VGA_BUFFER_ROWS * 2;
const VGA_BUFFER_ADDR: *mut () = 0xb8000 as *mut ();

/// Depending on the setup, the bright bit of background color may be
/// used as the blink bit.
/// See https://en.wikipedia.org/wiki/VGA_text_mode#endnote_text_buffer_1
#[allow(dead_code)]
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,

    // Bright colors or blink screen char.
    DarkGray = Self::Black as u8 | BRIGHT_BIT,
    LightBlue = Self::Blue as u8 | BRIGHT_BIT,
    LightGreen = Self::Green as u8 | BRIGHT_BIT,
    LightCyan = Self::Cyan as u8 | BRIGHT_BIT,
    LightRed = Self::Red as u8 | BRIGHT_BIT,
    Pink = Self::Magenta as u8 | BRIGHT_BIT,
    Yellow = Self::Brown as u8 | BRIGHT_BIT,
    White = Self::LightGray as u8 | BRIGHT_BIT,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ColorCode(u8);

impl ColorCode {
    pub const fn new(foreground: Color, background: Color) -> Self {
        Self(foreground as u8 | (background as u8) << 4)
    }

    #[allow(dead_code)]
    pub fn blink(self) -> Self {
        Self(self.0 | BLINK_BIT)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScreenChar {
    ascii_char: u8,
    color_code: ColorCode,
}

impl ScreenChar {
    #[allow(non_upper_case_globals)]
    pub const Blank: Self = Self::new(b' ', ColorCode::new(Color::Black, Color::Black));

    pub const fn new(ascii_char: u8, color_code: ColorCode) -> Self {
        Self {
            ascii_char,
            color_code,
        }
    }
}

// TODO:
// It's not good. It's protected by convention, not the type system.
/// Wrapper that indicates inner should not be written directly
/// without using volatile.
#[repr(transparent)]
struct Volatile<T>(T);

/// Type alias for non-volatile buffer row.
/// It's easier to use for the users of VgaBuffer.
type VgaBufferRow = [ScreenChar; VGA_BUFFER_COLUMNS];

// I prefer not to depends on an outside crate unless absolutely
// neccessary, so I don't use `volatile` crate here. Instead, I
// wrap them by myself.
#[repr(transparent)]
struct VgaBuffer([[Volatile<ScreenChar>; VGA_BUFFER_COLUMNS]; VGA_BUFFER_ROWS]);

impl VgaBuffer {
    /// Read a ScreenChar to the VGA buffer.
    /// # Panics
    /// Panics if row or col goes outside of the screen.
    #[allow(dead_code)]
    pub fn read_char(&self, row: usize, col: usize) -> ScreenChar {
        // Safety: self.0[row][col] will panics otherwise.
        unsafe { core::ptr::read_volatile(&self.0[row][col]).0 }
    }

    /// Write a ScreenChar to the VGA buffer.
    /// # Panics
    /// Panics if row or col goes outside of the screen.
    pub fn write_char(&mut self, row: usize, col: usize, ch: ScreenChar) {
        // Safety: self.0[row][col] will panics otherwise.
        unsafe {
            core::ptr::write_volatile(&mut self.0[row][col], Volatile(ch));
        }
    }

    /// Read a row at idx.
    /// # Panics
    /// Panics if idx goes outside of the screen
    pub fn read_row(&self, idx: usize) -> VgaBufferRow {
        // Safety: self.0[idx] will panics otherwise.
        unsafe { core::ptr::read_volatile(&self.0[idx] as *const _ as *const VgaBufferRow) }
    }

    /// Write a row at idx.
    /// # Panics
    /// Panics if idx goes outside of the screen
    pub fn write_row(&mut self, idx: usize, row: VgaBufferRow) {
        // Safety: self.0[idx] will panics otherwise.
        unsafe {
            core::ptr::write_volatile(&mut self.0[idx] as *mut _ as *mut VgaBufferRow, row);
        }
    }
}

pub struct Screen {
    row: usize,
    col: usize,
    buffer: &'static mut VgaBuffer,

    color_code: ColorCode,
}

impl Screen {
    fn new() -> Self {
        // Safety:
        // This is the vga buffer and we are the only user.
        let buffer = unsafe { &mut *(VGA_BUFFER_ADDR as *mut VgaBuffer) };

        Self {
            // This has a benefit that we know it will print to the last line,
            // which is convenient for writing tests.
            row: VGA_BUFFER_ROWS - 1,
            col: 0,
            buffer,
            color_code: ColorCode::new(Color::Yellow, Color::Black),
        }
    }

    /// Print a char on the current position. Add a new line if
    /// we hit the right boundary. Move all lines up if we are
    /// already at the bottom.
    ///
    /// If the char isn't printable, print 0xfe instead.
    ///
    /// Caveat:
    /// - We treat '\r' as '\r' and '\n' as '\r\n'.
    pub fn put_char(&mut self, ch: u8) {
        // Sanity check.
        assert!(self.col <= VGA_BUFFER_COLUMNS);
        assert!(self.row <= VGA_BUFFER_ROWS);

        if self.col == VGA_BUFFER_COLUMNS {
            self.new_line();
        }
        match ch {
            b'\n' => self.new_line(),
            b'\r' => self.col = 0,
            mut byte => {
                // Unprintable char
                if !(b' '..=b'~').contains(&byte) {
                    byte = 0xfe;
                }
                let ch = ScreenChar::new(byte, self.color_code);
                self.buffer.write_char(self.row, self.col, ch);
                self.col += 1;
            }
        };
    }

    /// Print each char in `s`.
    /// See [`put_char`] for details
    pub fn puts(&mut self, s: &str) {
        for ch in s.bytes() {
            self.put_char(ch);
        }
    }

    /// Add a new line below the current position. If we are
    /// already at the bottom, move all rows up and discard
    /// the first row.
    pub fn new_line(&mut self) {
        if self.row + 1 < VGA_BUFFER_ROWS {
            self.row += 1;
            self.col = 0;
            return;
        }
        // Move all rows up.
        // TODO: add the discarded line to history.
        for r in 0..(VGA_BUFFER_ROWS - 1) {
            let lower_row = self.buffer.read_row(r + 1);
            self.buffer.write_row(r, lower_row);
        }
        // Clear the last row.
        self.buffer
            .write_row(VGA_BUFFER_ROWS - 1, [ScreenChar::Blank; VGA_BUFFER_COLUMNS]);

        // self.row remains unchanged.
        self.col = 0;
    }
}

impl core::fmt::Write for Screen {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.puts(s);
        Ok(())
    }
}

lazy_static! {
    pub static ref SCREEN: SpinLock<Screen> = SpinLock::new(Screen::new());
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::screen::_print(::core::format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n");
    };
    ($($arg:tt)*) => {
        $crate::print!("{}\n", ::core::format_args!($($arg)*));
    };
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    crate::interrupts::without_interrupts(
        || SCREEN.lock().write_fmt(args).unwrap()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_printx_no_panic() {
        println!();
        println!("1");

        print!("1");
        print!("1\r");
        print!("1\r\n");
        print!("1\n1");

        for _ in 0..100 {
            println!("1");
        }
        for _ in 0..25 {
            println!("1");
        }
        for _ in 0..1024 {
            print!("1");
        }
    }

    #[test_case]
    fn test_println_output() {
        // Force a new line.
        println!();
        let s = "Some test string that fits on a single line";
        println!("{}", s);
        let screen = SCREEN.lock();
        for (col, ch) in s.chars().enumerate() {
            let screen_char = screen.buffer.read_char(VGA_BUFFER_ROWS - 2, col);
            assert_eq!(char::from(screen_char.ascii_char), ch);
        }
    }
}
