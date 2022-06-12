#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]
#![feature(custom_test_frameworks)]
#![test_runner(fyos::test_runner)]
#![reexport_test_harness_main = "test_main"]

use fyos::println;

static HELLO: &str = "Hello World!";
static MORNING: &str = "Morning! Nice day for fishing ain't it?";

fn main() {
    println!("{}\n", HELLO);
    for i in 1.. {
        println!("{} - {}", MORNING, i);
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    #[cfg(test)]
    test_main();

    #[cfg(not(test))]
    main();

    loop {
        core::hint::spin_loop();
    }
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{}", info);
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    fyos::test_panic_handler(info);
}
