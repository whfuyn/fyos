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
        println!("\n# Case {}", i);
        test();
    }
    exit_qemu(QemuExitCode::Success);
}

#[test_case]
fn trivial_assertion() {
    println!("trivial assertion..");
    assert_eq!(1, 1);
    println!("easy!");
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

/// Tell QEMU we are about to exit.
pub fn exit_qemu(exit_code: QemuExitCode) -> ! {
    let exit_code = exit_code as u32;
    // SAFETY:
    // I have no idea what I'm doing, but I guess this won't burn my CPU.
    // So it's safe, isn't it?:p
    unsafe {
        // See https://doc.rust-lang.org/nightly/rust-by-example/unsafe/asm.html
        core::arch::asm! {
            // Use the x register modifier to use 32-bit register.
            "out 0xfe, {exit_code:x}",
            exit_code = in(reg) exit_code,
            options(noreturn)
        };
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
