mod idt;

use core::fmt;
pub use idt::init_idt;

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
        #[naked]
        extern "C" fn wrapper() -> ! {
            // TODO: how to specify clobbered rdi? Is this asm correct?
            // SAFETY:
            // * Called as a interrupt handler
            unsafe {
                ::core::arch::asm!(
                    // We need to save the clobbered rdi as it's a callee-saved register.
                    // It doesn't compile to specify clobbered registers in naked fn,
                    // so we have to do it manually.
                    // Notice that this also makes the stack pointer aligns to 16.
                    "push rdi",
                    // Read the addr of the interrupt stack frame, and pass it to
                    // the handler as the first arg.
                    // Notice that we need to add 8 to make up the space used by rdi.
                    "mov rdi, rsp",
                    "add rdi, 8",
                    "call {}",
                    // Restore the clobbered rdi.
                    "pop rdi",
                    "iretq",
                    // Did I do it right?
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
        #[naked]
        extern "C" fn wrapper() -> ! {
            unsafe {
                ::core::arch::asm!(
                    // Put error code to rax
                    "pop rax",
                    // Load stack frame to rdx
                    "mov rdx, rsp",
                    // Save rdi and rsi
                    "push rdi",
                    "push rsi",
                    // Align stack pointer to 16
                    "sub rsp, 8",
                    // Pass stack frame and error code as args
                    "mov rdi, rdx",
                    "mov rsi, rax",
                    "call {}",
                    // Undo stack pointer alignment
                    "add rsp, 8",
                    // Restore rdi and rsi
                    "pop rsi",
                    "pop rdi",
                    "iretq",
                    sym $name,
                    options(noreturn)
                );
            }
        }
        wrapper
    }};
}
