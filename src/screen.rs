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

type VgaBufferRow = [ScreenChar; VGA_BUFFER_COLUMNS];

/// I prefer not to depends on an outside crate unless absolutely
/// neccessary, so I don't use `volatile` crate here. Instead, I
/// wraps those volatile ops inside here.
#[repr(transparent)]
struct VgaBuffer([VgaBufferRow; VGA_BUFFER_ROWS]);

impl VgaBuffer {
    /// Write a ScreenChar to the VGA buffer.
    /// # Panics
    /// Panics if row or col goes outside of the screen.
    pub fn write_char(&mut self, row: usize, col: usize, ch: ScreenChar) {
        // SAFETY: self.0[row][col] will panics otherwise.
        unsafe {
            core::ptr::write_volatile(&mut self.0[row][col], ch);
        }
    }

    /// Write a row at idx.
    /// # Panics
    /// Panics if idx goes outside of the screen
    pub fn write_row(&mut self, idx: usize, row: VgaBufferRow) {
        // SAFETY: self.0[idx] will panics otherwise.
        unsafe {
            core::ptr::write_volatile(&mut self.0[idx], row);
        }
    }

    /// Read a row at idx.
    /// # Panics
    /// Panics if idx goes outside of the screen
    pub fn read_row(&self, idx: usize) -> VgaBufferRow {
        // SAFETY: self.0[idx] will panics otherwise.
        unsafe { core::ptr::read_volatile(&self.0[idx]) }
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
        // SAFETY:
        // This is the vga buffer and we are the only user.
        let buffer = unsafe { &mut *(VGA_BUFFER_ADDR as *mut VgaBuffer) };

        Self {
            row: 0,
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
        self.clear_row(VGA_BUFFER_ROWS - 1);

        // self.row remains unchanged.
        self.col = 0;
    }

    /// Fill the row at idx with blank chars.
    /// # Panics
    /// Panics if idx goes outside of the screen.
    pub fn clear_row(&mut self, idx: usize) {
        self.buffer
            .write_row(idx, [ScreenChar::Blank; VGA_BUFFER_COLUMNS]);
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
    ($($arg:tt)*) => {{
        $crate::screen::_print(::core::format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! println {
    () => {{
        $crate::print!("\n");
    }};
    ($($arg:tt)*) => {{
        $crate::print!("{}\n", ::core::format_args!($($arg)*));
    }};
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    SCREEN.lock().write_fmt(args).unwrap();
}
