#![no_std]
#![no_main]

mod vga_buffer;

use core::panic::PanicInfo;
use vga_buffer::VgaBuffer;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

static HELLO: &[u8] = b"Hello World!";

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut vga_buffer = VgaBuffer::new();
    for (j, &ch) in HELLO.iter().enumerate() {
        vga_buffer.put_char(1, j, ch.into());
    }
    vga_buffer.sync();

    loop {}
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
