#![no_std]
#![no_main]

use fyos::{exit_qemu, serial_print, serial_println, QemuExitCode};

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    serial_println!("[OK]");
    exit_qemu(QemuExitCode::Success);
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    should_fail();
    serial_println!("[Test did not panic]");
    exit_qemu(QemuExitCode::Failed);
}

fn should_fail() {
    serial_print!("should_panic::should_fail...\t");
    assert_eq!(0, 1);
}
