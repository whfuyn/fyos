#![no_std]
#![no_main]
#![feature(type_name_of_val)]

use fyos::{bit_field::BitField, exit_qemu, serial_print, serial_println, QemuExitCode};

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    serial_println!("[OK]");
    exit_qemu(QemuExitCode::Success);
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    test_bit_field_range_protect();
    serial_println!("[Test did not panic]");
    exit_qemu(QemuExitCode::Failed);
}

fn test_bit_field_range_protect() {
    serial_print!(
        "{}...\t",
        core::any::type_name_of_val(&test_bit_field_range_protect)
    );
    let mut bits = 0;
    bits.set_bits(1..=2, 0b111);
}
