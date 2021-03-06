#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(fyos::test_runner)]
#![reexport_test_harness_main = "test_main"]

use fyos::println;
use fyos::x86_64;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    test_main();
    x86_64::hlt_loop();
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    fyos::test_panic_handler(info);
}

#[test_case]
fn test_println() {
    println!("test_println ok");
}
