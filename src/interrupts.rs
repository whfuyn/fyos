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
        const _: extern "C" fn(&InterruptStackFrame) = $name;
        $crate::raw_handler!(@INNER $name)
    }};
    ($name: ident -> !) => {{
        // Signature check
        const _: extern "C" fn(&InterruptStackFrame) -> ! = $name;
        $crate::raw_handler!(@INNER $name)
    }};
    (@INNER $name: ident) => {{
        #[naked]
        extern "C" fn wrapper() -> ! {
            // TODO: how to specify clobber rdi? Is this asm correct?
            // SAFETY:
            // * Called as a interrupt handler
            unsafe {
                ::core::arch::asm!(
                    // Read the addr of the interrupt stack frame,
                    // and pass it to the handler as the first arg.
                    "mov rdi, rsp",
                    // Align the stack pointer.
                    // See https://os.phil-opp.com/better-exception-messages/#fixing-the-alignment
                    "sub rsp, 8",
                    "call {}",
                    "add rsp, 8",
                    "iretq",
                    // Did I do it right? Should I specify clobber_abi("C")? How?
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
        const _: extern "C" fn(&InterruptStackFrame, ErrorCode) = $name;
        $crate::raw_handler_with_error_code!(@INNER $name)
    }};
    ($name: ident -> !) => {{
        const _: extern "C" fn(&InterruptStackFrame, ErrorCode) -> ! = $name;
        $crate::raw_handler_with_error_code!(@INNER $name)
    }};
    (@INNER $name: ident) => {{
        #[naked]
        extern "C" fn wrapper() -> ! {
            unsafe {
                ::core::arch::asm!(
                    // Pop error code to be used as the second arg.
                    "pop rsi",
                    "mov rdi, rsp",
                    "sub rsp, 8",
                    "call {}",
                    "add rsp, 8",
                    "iretq",
                    sym $name,
                    options(noreturn)
                );
            }
        }
        wrapper
    }};
}
