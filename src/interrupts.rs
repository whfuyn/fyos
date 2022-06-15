mod idt;

pub use idt::init_idt;

#[macro_export]
macro_rules! raw_handler {
    ($name: ident) => {{
        // Signature check
        const _: extern "C" fn(&InterruptStackFrame) -> ! = $name;
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
                    // Align the stack pointer
                    "sub rsp, 8",
                    "call {}",
                    // Did I do it right?
                    sym $name,
                    options(noreturn)
                )
            }
        }
        wrapper
    }};
}
