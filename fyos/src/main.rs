#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]

mod console;
mod vga_buffer;

use core::panic::PanicInfo;
// use vga_buffer::VgaBuffer;
use console::Console;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

static HELLO: &[u8] = b"Hello World!\n";
static MORNING: &[u8] = b"Morning! Nice day for fishing ain't it! Hu ha!\n";

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut console = Console::new();
    console.puts(HELLO);
    loop {
        console.puts(MORNING);
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
