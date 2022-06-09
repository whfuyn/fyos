#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]
#![feature(format_args_nl)]

mod konsole;
mod sync;
mod vga_buffer;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

static HELLO: &str = "Hello World!";
static MORNING: &str = "Morning! Nice day for fishing ain't it!";

#[derive(Debug)]
enum Error {
    FmtError(core::fmt::Error),
}

impl From<core::fmt::Error> for Error {
    fn from(e: core::fmt::Error) -> Self {
        Self::FmtError(e)
    }
}

fn main() -> Result<(), Error> {
    kprintln!("{}\n", HELLO)?;
    kprintln!("{}\n", MORNING)?;
    kprint!("{}", MORNING)?;
    Ok(())
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    main().unwrap();

    loop {
        core::hint::spin_loop();
    }
}

// bootloader 0.10 doesn't work.
// Nothing shows up and qemu keeps rebooting.

// bootloader::entry_point!(_start);

// pub fn _start(_: &'static mut bootloader::BootInfo) -> ! {
//     let vga_buffer  = 0xb8000 as *mut u8;
//     for (i, &c) in HELLO.iter().enumerate() {
//         unsafe {
//             *vga_buffer.offset(i as isize * 2) = c;
//             *vga_buffer.offset(i as isize * 2 + 1) = 0xb;
//         }
//     }

//     loop {}
// }
