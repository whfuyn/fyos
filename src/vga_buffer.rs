mod color {
    // TODO: remove it
    #![allow(dead_code)]

    const BRIGHT_BIT: u8 = 1 << 3;
    const BLINK_BIT: u8 = 1 << 7;

    #[repr(u8)]
    #[derive(Clone, Copy)]
    pub enum BackgroundColor {
        Black = 0,
        Blue = 1,
        Green = 2,
        Cyan = 3,
        Red = 4,
        Magenta = 5,
        Brown = 6,
        LightGray = 7,
    }

    #[repr(u8)]
    #[derive(Clone, Copy)]
    pub enum ForegroundColor {
        Black = BackgroundColor::Black as u8,
        Blue = BackgroundColor::Blue as u8,
        Green = BackgroundColor::Green as u8,
        Cyan = BackgroundColor::Cyan as u8,
        Red = BackgroundColor::Red as u8,
        Magenta = BackgroundColor::Magenta as u8,
        Brown = BackgroundColor::Brown as u8,
        LightGray = BackgroundColor::LightGray as u8,

        DarkGray = Self::Black as u8 | BRIGHT_BIT,
        LightBlue = Self::Blue as u8 | BRIGHT_BIT,
        LightGreen = Self::Green as u8 | BRIGHT_BIT,
        LightCyan = Self::Cyan as u8 | BRIGHT_BIT,
        LightRed = Self::Red as u8 | BRIGHT_BIT,
        Pink = Self::Magenta as u8 | BRIGHT_BIT,
        Yellow = Self::Brown as u8 | BRIGHT_BIT,
        White = Self::LightGray as u8 | BRIGHT_BIT,
    }

    #[derive(Clone, Copy)]
    pub struct Color {
        pub fg: ForegroundColor,
        pub bg: BackgroundColor,
        // This may behave like the bright bit.
        // See https://en.wikipedia.org/wiki/VGA_text_mode#endnote_text_buffer_1
        pub blink: bool,
    }

    impl Color {
        #![allow(non_upper_case_globals)]

        pub const White: Self = Self {
            fg: ForegroundColor::White,
            bg: BackgroundColor::Black,
            blink: false,
        };

        pub const Black: Self = Self {
            fg: ForegroundColor::Black,
            bg: BackgroundColor::Black,
            blink: false,
        };

        pub const Yellow: Self = Self {
            fg: ForegroundColor::Yellow,
            bg: BackgroundColor::Black,
            blink: false,
        };
    }

    impl From<Color> for u8 {
        fn from(clr: Color) -> Self {
            let blink_bit = if clr.blink { BLINK_BIT } else { 0 };
            clr.fg as u8 | ((clr.bg as u8) << 4) | blink_bit
        }
    }

    impl From<ForegroundColor> for Color {
        fn from(fg: ForegroundColor) -> Self {
            Self {
                fg,
                bg: BackgroundColor::Black,
                blink: false,
            }
        }
    }
}

pub use color::*;

#[derive(Clone, Copy)]
pub struct VgaChar {
    code_point: u8,
    color: Color,
}

impl VgaChar {
    pub const fn new(code_point: u8, color: Color) -> Self {
        Self {
            code_point,
            color,
        }
    }

    #[inline]
    pub fn code_point(&self) -> u8 {
        self.code_point
    }

    pub fn blink(mut self) -> Self {
        self.color.blink = true;
        self
    }
}

impl Default for VgaChar {
    fn default() -> Self {
        Self {
            code_point: b' ',
            color: Color::Black,
        }
    }
}

impl From<u8> for VgaChar {
    fn from(code_point: u8) -> Self {
        Self {
            code_point,
            color: Color::White,
        }
    }
}

impl From<VgaChar> for [u8; 2] {
    fn from(ch: VgaChar) -> Self {
        [ch.code_point, ch.color.into()]
    }
}

pub const VGA_BUFFER_ROWS: usize = 25;
pub const VGA_BUFFER_COLUMNS: usize = 80;
const VGA_BUFFER_ADDR: *mut u8 = 0xb8000 as *mut u8;

const VGA_COL_OFFSET: usize = 2;
const VGA_ROW_OFFSET: usize = VGA_BUFFER_COLUMNS * VGA_COL_OFFSET;

type VgaRow = [VgaChar; VGA_BUFFER_COLUMNS];

// Design question:
// Should I use an internal buffer or just write to the vga memory?
pub struct VgaBuffer {
    pub buffer: [VgaRow; VGA_BUFFER_ROWS],
}

impl VgaBuffer {
    pub const fn new() -> Self {
        let blank = VgaChar {
            code_point: b' ',
            color: Color::Yellow,
        };
        let buffer = [[blank; VGA_BUFFER_COLUMNS]; VGA_BUFFER_ROWS];
        Self { buffer }
    }

    /// Write a vga char to the vga's buffer.
    /// # Panics
    /// Panics if the row and col goes outside of the screen.
    fn write_vga_char<C: Into<VgaChar>>(rows: usize, cols: usize, ch: C) {
        let vga_char: [u8; 2] = ch.into().into();
        let offset = rows * VGA_ROW_OFFSET + cols * VGA_COL_OFFSET;

        assert!(rows < VGA_BUFFER_ROWS);
        assert!(cols < VGA_BUFFER_COLUMNS);
        // SAFETY:
        // The addr we are writing to is indeed inside the vga's buffer,
        // which we have checked above.
        unsafe {
            VGA_BUFFER_ADDR.add(offset).write_volatile(vga_char[0]);
            VGA_BUFFER_ADDR.add(offset + 1).write_volatile(vga_char[1]);
        }
    }

    /// Write the whole buffer to vga.
    pub fn flush(&self) {
        for (i, row) in self.buffer.iter().enumerate() {
            for (j, ch) in row.iter().enumerate() {
                Self::write_vga_char(i, j, *ch);
            }
        }
    }
}
