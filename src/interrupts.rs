pub mod idt;

use core::fmt;
pub use idt::init_idt;
pub use crate::pic::ChainedPics;
use crate::spinlock::SpinLock;


pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: SpinLock<ChainedPics> = SpinLock::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct ErrorCode(u64);

impl fmt::LowerHex for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

impl fmt::UpperHex for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::UpperHex::fmt(&self.0, f)
    }
}

#[macro_export]
macro_rules! raw_handler {
    ($name: ident) => {{
        // Signature check
        const _: extern "C" fn(&$crate::x86_64::InterruptStackFrame) = $name;
        $crate::raw_handler!(@INNER $name)
    }};
    ($name: ident -> !) => {{
        // Signature check
        const _: extern "C" fn(&$crate::x86_64::InterruptStackFrame) -> ! = $name;
        $crate::raw_handler!(@INNER $name)
    }};
    (@INNER $name: ident) => {{
        // Safety:
        // * Must be used as an interrupt handler.
        #[naked]
        unsafe extern "C" fn wrapper() -> ! {
            // Safety:
            // * All scratch registers are saved and restored.
            // * Handler signature has been checked above.
            unsafe {
                ::core::arch::asm!(
                    // Save scratch registers
                    "push rax",
                    "push rcx",
                    "push rdx",
                    "push rsi",
                    "push rdi",
                    "push r8",
                    "push r9",
                    "push r10",
                    "push r11",
                    // Read the original addr of the interrupt stack frame,
                    // and pass it as the first argument to the handler.
                    // Be careful not to change any arithmetic flags.
                    "lea rdi, [rsp + 0x48]",
                    // Notice that we've pushed 9 registers onto stack, which
                    // fortunately also make rsp align to 16.
                    "call {}",
                    // Restore scratch registers
                    "pop r11",
                    "pop r10",
                    "pop r9",
                    "pop r8",
                    "pop rdi",
                    "pop rsi",
                    "pop rdx",
                    "pop rcx",
                    "pop rax",
                    "iretq",
                    sym $name,
                    options(noreturn)
                )
            }
        }
        wrapper
    }};
}

#[macro_export]
macro_rules! raw_handler_with_error_code {
    ($name: ident) => {{
        const _: extern "C" fn(&$crate::x86_64::InterruptStackFrame, $crate::interrupts::ErrorCode) = $name;
        $crate::raw_handler_with_error_code!(@INNER $name)
    }};
    ($name: ident -> !) => {{
        const _: extern "C" fn(&$crate::x86_64::InterruptStackFrame, $crate::interrupts::ErrorCode) -> ! = $name;
        $crate::raw_handler_with_error_code!(@INNER $name)
    }};
    (@INNER $name: ident) => {{
        // Safety:
        // * Must be used as an interrupt handler which has an error code.
        #[naked]
        unsafe extern "C" fn wrapper() -> ! {
            // Safety:
            // * All scratch registers are saved and restored.
            // * Handler signature has been checked above.
            unsafe {
                ::core::arch::asm!(
                    // Save scratch registers
                    "push rax",
                    "push rcx",
                    "push rdx",
                    "push rsi",
                    "push rdi",
                    "push r8",
                    "push r9",
                    "push r10",
                    "push r11",
                    // Read the original addr of the interrupt stack frame,
                    // and pass it as the first argument to the handler.
                    // Be careful not to change any arithmetic flags.
                    "lea rdi, [rsp + 0x50]",  // 8 * (9 registers + 1 error code)
                    // Load the error code to rsi as the second argument.
                    "mov rsi, [rsp + 0x48]",
                    // Notice that we've pushed 9 registers onto stack, which
                    // fortunately also make rsp align to 16.
                    "call {}",
                    // Restore scratch registers
                    "pop r11",
                    "pop r10",
                    "pop r9",
                    "pop r8",
                    "pop rdi",
                    "pop rsi",
                    "pop rdx",
                    "pop rcx",
                    "pop rax",
                    // Pop error code
                    "lea rsp, [rsp + 8]",
                    "iretq",
                    sym $name,
                    options(noreturn)
                )
            }
        }
        wrapper
    }};
}
