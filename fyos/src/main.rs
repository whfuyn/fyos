#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

static HELLO: &[u8] = b"Hello World!";

#[no_mangle]
pub extern fn _start() -> ! {
    let vga_buffer  = 0xb8000 as *mut u8;
    for (i, &c) in HELLO.iter().enumerate() {
        unsafe {
            *vga_buffer.offset(i as isize * 2) = c;
            *vga_buffer.offset(i as isize * 2 + 1) = 0xb;
        }
    }
    
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
