pub mod idt;

pub use idt::init_idt;
pub use crate::pic::ChainedPics;

use core::fmt;
use core::marker::PhantomData;
use crate::spinlock::SpinLock;
use crate::x86_64::VirtAddr;


pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: SpinLock<ChainedPics> = SpinLock::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });


pub type HandlerFunc = extern "x86-interrupt" fn(InterruptStackFrame);
pub type DivergingHandlerFunc = extern "x86-interrupt" fn(InterruptStackFrame) -> !;
pub type HandlerFuncWithErrorCode =
    extern "x86-interrupt" fn(InterruptStackFrame, ErrorCode);
pub type DivergingHandlerFuncWithErrorCode =
    extern "x86-interrupt" fn(InterruptStackFrame, ErrorCode) -> !;

pub type RawHandlerFunc = extern "C" fn(&InterruptStackFrame);
pub type RawDivergingHandlerFunc = extern "C" fn(&InterruptStackFrame) -> !;
pub type RawHandlerFuncWithErrorCode = extern "C" fn(&InterruptStackFrame, ErrorCode);
pub type RawDivergingHandlerFuncWithErrorCode = extern "C" fn(&InterruptStackFrame, ErrorCode) -> !;

pub type PageFaultHandlerFunc =
    extern "x86-interrupt" fn(InterruptStackFrame, PageFaultErrorCode);
pub type RawPageFaultHandlerFunc =
    extern "C" fn(&InterruptStackFrame, PageFaultErrorCode);

pub trait HandlerFn {
    type Handler;
    type RawHandler;
}

impl HandlerFn for HandlerFunc {
    type Handler = Self;
    type RawHandler = RawHandlerFunc;
}

impl HandlerFn for DivergingHandlerFunc {
    type Handler = Self;
    type RawHandler = RawDivergingHandlerFunc;
}

impl HandlerFn for HandlerFuncWithErrorCode {
    type Handler = Self;
    type RawHandler = RawHandlerFuncWithErrorCode;
}

impl HandlerFn for DivergingHandlerFuncWithErrorCode {
    type Handler = Self;
    type RawHandler = RawDivergingHandlerFuncWithErrorCode;
}

impl HandlerFn for PageFaultHandlerFunc {
    type Handler = Self;
    type RawHandler = RawPageFaultHandlerFunc;
}

impl HandlerFn for RawHandlerFunc {
    type Handler = HandlerFunc;
    type RawHandler = Self;
}

impl HandlerFn for RawDivergingHandlerFunc {
    type Handler = DivergingHandlerFunc;
    type RawHandler = Self;
}

impl HandlerFn for RawHandlerFuncWithErrorCode {
    type Handler = HandlerFuncWithErrorCode;
    type RawHandler = Self;
}

impl HandlerFn for RawDivergingHandlerFuncWithErrorCode {
    type Handler = DivergingHandlerFuncWithErrorCode;
    type RawHandler = Self;
}

impl HandlerFn for RawPageFaultHandlerFunc {
    type Handler = PageFaultHandlerFunc;
    type RawHandler = Self;
}

pub struct RawHandler<F: HandlerFn> {
    /// Wrapped raw handler fn
    handler: unsafe extern "C" fn() -> !,
    /// To preserve type info
    _phantom: PhantomData<F>,
}

impl<F: HandlerFn> RawHandler<F> {
    pub const unsafe fn new(handler: unsafe extern "C" fn() -> !, _phantom: PhantomData<F>) -> Self {
        Self { handler, _phantom }
    }
}

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

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct PageFaultErrorCode(u64);

impl fmt::LowerHex for PageFaultErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

impl fmt::UpperHex for PageFaultErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::UpperHex::fmt(&self.0, f)
    }
}

#[macro_export]
macro_rules! raw_handler {
    ($name: ident) => {{
        // Signature check
        const _: $crate::interrupts::RawHandlerFunc = $name;
        #[allow(unused_unsafe)]
        unsafe {
            $crate::interrupts::RawHandler::new(
                $crate::raw_handler!(@INNER $name),
                ::core::marker::PhantomData::<$crate::interrupts::RawHandlerFunc>,
            )
        }
    }};
    ($name: ident -> !) => {{
        // Signature check
        const _: $crate::interrupts::RawDivergingHandlerFunc = $name;
        #[allow(unused_unsafe)]
        unsafe {
            $crate::interrupts::RawHandler::new(
                $crate::raw_handler!(@INNER $name),
                ::core::marker::PhantomData::<$crate::interrupts::RawDivergingHandlerFunc>,
            )
        }
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
        // Signature check
        const _: $crate::interrupts::RawHandlerFuncWithErrorCode = $name;
        #[allow(unused_unsafe)]
        unsafe {
            $crate::interrupts::RawHandler::new(
                $crate::raw_handler_with_error_code!(@INNER $name),
                ::core::marker::PhantomData::<$crate::interrupts::RawHandlerFuncWithErrorCode>,
            )
        }
    }};
    ($name: ident -> !) => {{
        const _: $crate::interrupts::RawDivergingHandlerFuncWithErrorCode = $name;
        #[allow(unused_unsafe)]
        unsafe {
            $crate::interrupts::RawHandler::new(
                $crate::raw_handler_with_error_code!(@INNER $name),
                ::core::marker::PhantomData::<$crate::interrupts::RawDivergingHandlerFuncWithErrorCode>,
            )
        }
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

#[macro_export]
macro_rules! raw_page_fault_handler {
    ($name: ident) => {{
        const _: $crate::interrupts::RawPageFaultHandlerFunc = $name;
        #[allow(unused_unsafe)]
        unsafe {
            $crate::interrupts::RawHandler::new(
                $crate::raw_handler_with_error_code!(@INNER $name),
                ::core::marker::PhantomData::<$crate::interrupts::RawPageFaultHandlerFunc>,
            )
        }
    }};
}

// TODO: impl dref and unsafe get_mut
/// Wrapper that ensures no accidental modification of the interrupt stack frame.(?)
#[derive(Debug)]
#[repr(C)]
pub struct InterruptStackFrame {
    value: InterruptStackFrameValue,
}

impl core::ops::Deref for InterruptStackFrame {
    type Target = InterruptStackFrameValue;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct InterruptStackFrameValue {
    pub instruction_pointer: VirtAddr,
    pub code_segment: u64,
    pub cpu_flags: u64,
    pub stack_pointer: VirtAddr,
    pub stack_segment: u64,
}
