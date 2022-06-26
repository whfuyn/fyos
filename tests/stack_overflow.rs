#![no_std]
#![no_main]
#![feature(type_name_of_val)]
#![feature(naked_functions)]
#![feature(asm_sym)]

use fyos::{
    exit_qemu,
    gdt::{init as init_gdt, DOUBLE_FAULT_IST_INDEX},
    interrupts::{
        idt::{Exception, InterruptDescriptorTable},
        ErrorCode,
    },
    lazy_static, raw_handler_with_error_code, serial_print, serial_println,
    interrupts::InterruptStackFrame,
    QemuExitCode,
};

lazy_static! {
    static ref TEST_IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        unsafe {
            idt.set_raw_handler_with_error_code(
                Exception::DoubleFault,
                raw_handler_with_error_code!(raw_double_fault_handler -> !),
            )
            .set_stack_index(DOUBLE_FAULT_IST_INDEX);
        }
        idt
    };
}

extern "C" fn raw_double_fault_handler(_: &InterruptStackFrame, _: ErrorCode) -> ! {
    serial_println!("[OK]");
    exit_qemu(QemuExitCode::Success);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    serial_println!("[OK]");
    exit_qemu(QemuExitCode::Success);
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    test_stack_overflow();
    serial_println!("[Test did not panic]");
    exit_qemu(QemuExitCode::Failed);
}

fn test_stack_overflow() {
    init_gdt();
    TEST_IDT.load();
    #[allow(unconditional_recursion)]
    fn overflow() {
        let i = 42;
        overflow();
        // Prevent tail recursion optimization
        unsafe {
            (&i as *const i32).read_volatile();
        }
    }
    serial_print!("{}...\t", core::any::type_name_of_val(&test_stack_overflow));
    overflow();
}
