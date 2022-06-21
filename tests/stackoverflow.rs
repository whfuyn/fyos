#![no_std]
#![no_main]
#![feature(type_name_of_val)]

use fyos::{exit_qemu, init, serial_print, serial_println, QemuExitCode};

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    serial_println!("[OK]");
    exit_qemu(QemuExitCode::Success);
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    test_stackoverflow();
    serial_println!("[Test did not panic]");
    exit_qemu(QemuExitCode::Failed);
}

fn test_stackoverflow() {
    init();
    #[allow(unconditional_recursion)]
    fn overflow() {
        let i = 42;
        overflow();
        // Prevent tail recursion optimization
        unsafe {
            (&i as *const i32).read_volatile();
        }
    }
    serial_print!("{}...\t", core::any::type_name_of_val(&test_stackoverflow));
    overflow();
}
