#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]
#![feature(custom_test_frameworks)]

#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

mod lazy_static;
mod screen;
mod spinlock;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{}", info);
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(test)]
fn test_runner(tests: &[&dyn Fn()]) {
    println!("Running {} tests..", tests.len());

    for (i, test) in tests.iter().enumerate() {
        println!("\n# Case #{}", i);
        test();
    }

}

static HELLO: &str = "Hello World!";
static MORNING: &str = "Morning! Nice day for fishing ain't it?";

fn main() {
    println!("{}\n", HELLO);
    for i in 1.. {
        println!("{} - {}", MORNING, i);
    }
}

#[test_case]
fn trivial_assertion() {
    println!("trivial assertion..");
    assert_eq!(1, 1);
    println!("easy!");
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
