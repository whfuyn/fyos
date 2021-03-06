#![no_std]
#![cfg_attr(test, no_main)]
#![deny(unsafe_op_in_unsafe_fn)]
#![test_runner(test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![feature(naked_functions)]
#![feature(asm_sym)]

pub mod gdt;
pub mod interrupts;
pub mod lazy_static;
pub mod port;
pub mod pic;
pub mod screen;
pub mod serial;
pub mod spinlock;
pub mod x86_64;

// TODO: how to make it pub only to should-panic tests?
pub mod bit_field;

pub fn init() {
    gdt::init();
    interrupts::init();
    unsafe {
        interrupts::PICS.lock().initialize();
    }
    x86_64::enable_interrupt();
}

pub trait Testable {
    fn run(&self);
}

impl<F: Fn()> Testable for F {
    fn run(&self) {
        serial_print!("{} ...\t", core::any::type_name::<F>());
        self();
        serial_print!("[OK]");
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
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

pub fn test_panic_handler(info: &core::panic::PanicInfo) -> ! {
    serial_println!("[Failed]");
    serial_println!("{}", info);
    exit_qemu(QemuExitCode::Failed);
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

/// Tell QEMU we are about to exit.
pub fn exit_qemu(exit_code: QemuExitCode) -> ! {
    // Safety:
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

#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    test_main();
    x86_64::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    test_panic_handler(info);
}
