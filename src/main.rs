#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

mod lazy_static;
mod screen;
mod serial;
mod spinlock;

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
    serial_println!("[Failed]");
    serial_println!("{}", info);
    exit_qemu(QemuExitCode::Failed);
}

#[cfg(test)]
trait Testable {
    fn run(&self);
}

#[cfg(test)]
impl<F: Fn()> Testable for F {
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<F>());
        self();
        serial_print!("[OK]");
    }
}

#[cfg(test)]
fn test_runner(tests: &[&dyn Testable]) {
    if !tests.is_empty() {
        serial_println!("Running {} tests...", tests.len());
    } else {
        serial_println!("No test to run.");
    }

    for &test in tests.iter() {
        test.run();
        serial_println!();
    }
    exit_qemu(QemuExitCode::Success);
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

/// Tell QEMU we are about to exit.
pub fn exit_qemu(exit_code: QemuExitCode) -> ! {
    // SAFETY:
    // Write exit code to QEMU's isa-debug-exit device.
    unsafe {
        // See https://doc.rust-lang.org/nightly/rust-by-example/unsafe/asm.html
        core::arch::asm! {
            "out 0xf4, eax",
            in("eax") exit_code as u32,
            options(noreturn, nomem, nostack, preserves_flags)
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
