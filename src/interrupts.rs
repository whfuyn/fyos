pub mod idt;

pub use crate::pic::ChainedPics;

use core::fmt;
use core::marker::PhantomData;
use crate::spinlock::SpinLock;
use crate::x86_64::{self, VirtAddr};
use crate::lazy_static;
use crate::print;
use crate::println;
use crate::port::{ Port, PortRead };
use crate::serial_print;
use crate::serial_println;
use idt::InterruptDescriptorTable;


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

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        // Both handler and raw handler should work.
        idt.divide_error.set_raw_handler(raw_handler!(raw_divide_by_zero_handler));
        idt.breakpoint.set_handler(breakpoint_handler);
        idt.invalid_opcode.set_raw_handler(raw_handler!(raw_invalid_opcode_handler));
        // Safety:
        // * The stack index points to a valid stack in GDT.
        // * It's not used by other interrupt handler.
        unsafe {
            idt.double_fault
                .set_raw_handler(raw_handler_with_error_code!(raw_double_fault_handler -> !))
                .set_stack_index(crate::gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt.general_protection_fault
            .set_raw_handler(raw_handler_with_error_code!(raw_general_protection_fault_handler));
        idt.page_fault
            .set_raw_handler(raw_page_fault_handler!(raw_page_fault_handler));

        idt[InterruptIndex::Timer as usize]
            .set_raw_handler(raw_handler!(raw_timer_handler));
        idt[InterruptIndex::Keyboard as usize]
            .set_raw_handler(raw_handler!(raw_keyboard_handler));
        idt
    };
}

pub fn init() {
    IDT.load();
}

extern "C" fn raw_keyboard_handler(_stack_frame: &InterruptStackFrame) {
    use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};

    lazy_static! {
        static ref KEYBOARD: SpinLock<Keyboard<layouts::Us104Key, ScancodeSet1>> = 
            SpinLock::new(Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore));
    }

    let mut keyboard = KEYBOARD.lock();
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(ch) => print!("{ch}"),
                DecodedKey::RawKey(key) => print!("{key:?}"),
            }
        }
    }

    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Keyboard as u8);
    }
}

extern "C" fn raw_timer_handler(_stack_frame: &InterruptStackFrame) {
    print!(".");
    serial_print!(".");
    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Timer as u8);
    }
}

extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, error: ErrorCode) {
    serial_println!(
        "EXCEPTION: double fault with error code `{:#x}` at {:#x}\n{:#?}",
        error,
        stack_frame.instruction_pointer,
        stack_frame
    );
    x86_64::hlt_loop();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    serial_println!("Haoye! It's a breakpoint!");
    serial_println!(
        "At {:#x}\nStackFrame:\n{:#?}",
        stack_frame.instruction_pointer,
        stack_frame
    );
}

extern "C" fn raw_breakpoint_handler(stack_frame: &InterruptStackFrame) {
    serial_println!("Haoye! It's a breakpoint!");
    serial_println!(
        "At {:#x}\nStackFrame:\n{:#?}",
        stack_frame.instruction_pointer,
        stack_frame
    );
}

extern "C" fn raw_divide_by_zero_handler(stack_frame: &InterruptStackFrame) {
    serial_println!("EXCEPTION: divide-by-zero");
    serial_println!("{:#?}", stack_frame);
    x86_64::hlt_loop();
}

extern "C" fn raw_invalid_opcode_handler(stack_frame: &InterruptStackFrame) {
    serial_println!(
        "EXCEPTION: invalid opcode at {:#x}\n{:#?}",
        stack_frame.instruction_pointer,
        stack_frame
    );
    x86_64::hlt_loop();
}

extern "C" fn raw_double_fault_handler(stack_frame: &InterruptStackFrame, error: ErrorCode) -> ! {
    panic!(
        "EXCEPTION: double fault with error code `{:#x}` at {:#x}\n{:#?}",
        error, stack_frame.instruction_pointer, stack_frame
    );
}

extern "C" fn raw_general_protection_fault_handler(
    stack_frame: &InterruptStackFrame,
    error: ErrorCode,
) {
    serial_println!(
        "EXCEPTION: general protection fault with error code `{:#x}` at {:#x}\n{:#?}",
        error,
        stack_frame.instruction_pointer,
        stack_frame
    );
    x86_64::hlt_loop();
}

extern "C" fn raw_page_fault_handler(stack_frame: &InterruptStackFrame, error: PageFaultErrorCode) {
    serial_println!(
        "EXCEPTION: page fault with error code `{:#x}` at {:#x}\n{:#?}",
        error,
        stack_frame.instruction_pointer,
        stack_frame
    );
    x86_64::hlt_loop();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gdt;
    use crate::serial_println;
    use crate::x86_64;

    #[test_case]
    fn test_breakpoint_handler() {
        init();
        serial_println!("go!");
        x86_64::int3();
        serial_println!("haoye!");
    }

    // #[test_case]
    // fn test_divid_by_zero_handler() {
    //     init_idt();
    //     serial_println!("go!");
    //     x86_64::divide_by_zero();
    //     serial_println!("No haoye!");
    // }

    // #[test_case]
    // fn test_invalid_opcode_handler() {
    //     init_idt();
    //     serial_println!("go!");
    //     x86_64::ud2();
    //     serial_println!("No haoye!");
    // }

    // #[test_case]
    // fn test_page_fault_handler() {
    //     gdt::init();
    //     init_idt();
    //     serial_println!("go!");
    //     unsafe {
    //         *(0xdeadbeef as *mut u8) = 42;
    //     }
    //     serial_println!("No haoye!");
    // }

    // #[test_case]
    // fn test_double_fault_handler() {
    //     gdt::init();
    //     init_idt();
    //     divide_by_zero();
    //     serial_println!("No haoye!");
    // }

    #[test_case]
    fn test_timer_handler() {
        crate::init();
        serial_println!("start");
        loop {
            serial_print!("*");
            for _ in 0..10000{}
        }
    }
}