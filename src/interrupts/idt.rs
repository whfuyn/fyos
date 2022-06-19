use super::*;
use crate::bit_field::BitField;
use crate::lazy_static;
use crate::println;
use crate::raw_handler;
use crate::raw_handler_with_error_code;
use crate::serial_println;
use crate::x86_64::*;

/// x86_64 exception vector number.
#[derive(Debug, Clone, Copy)]
#[repr(usize)]
pub enum Exception {
    DivideByZero = 0,
    BreakPoint = 3,
    InvalidOpCode = 6,
    DoubleFault = 8,
    PageFault = 14,
}

lazy_static! {
    static ref IDT: Idt = {
        let mut idt = Idt::new();
        idt.set_raw_handler(
            Exception::DivideByZero,
            raw_handler!(raw_divide_by_zero_handler -> !),
        );
        // idt.set_handler(Exception::BreakPoint, breakpoint_handler);
        idt.set_raw_handler(Exception::BreakPoint, raw_handler!(raw_breakpoint_handler));
        idt.set_raw_handler(
            Exception::InvalidOpCode,
            raw_handler!(raw_invalid_opcode_handler -> !),
        );
        idt.set_raw_handler_with_error_code(Exception::DoubleFault, raw_handler_with_error_code!(raw_double_fault_handler -> !));
        idt.set_raw_handler(
            Exception::PageFault,
            raw_handler_with_error_code!(raw_page_fault_handler -> !),
        );
        idt
    };
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

extern "C" fn raw_divide_by_zero_handler(stack_frame: &InterruptStackFrame) -> ! {
    serial_println!("looks at the stack frame!");
    serial_println!("{:#?}", stack_frame);
    loop {
        core::hint::spin_loop();
    }
}

extern "C" fn raw_invalid_opcode_handler(stack_frame: &InterruptStackFrame) -> ! {
    serial_println!(
        "EXCEPTION: invalid opcode at {:#x}\n{:#?}",
        stack_frame.instruction_pointer,
        stack_frame
    );
    loop {
        core::hint::spin_loop();
    }
}

extern "C" fn raw_double_fault_handler(stack_frame: &InterruptStackFrame, error: ErrorCode) -> ! {
    panic!(
        "EXCEPTION: double fault with error code `{:#x}` at {:#x}\n{:#?}",
        error, stack_frame.instruction_pointer, stack_frame
    );
}

extern "C" fn raw_page_fault_handler(stack_frame: &InterruptStackFrame, error: ErrorCode) -> ! {
    serial_println!(
        "EXCEPTION: page fault with error code `{:#x}` at {:#x}\n{:#?}",
        error,
        stack_frame.instruction_pointer,
        stack_frame
    );
    loop {
        core::hint::spin_loop();
    }
}

pub fn init_idt() {
    IDT.load();
}

#[repr(transparent)]
pub struct Idt([Entry; 16]);

impl Idt {
    pub fn new() -> Idt {
        Idt([Entry::missing(); 16])
    }

    // We don't return a &mut EntryOptions because ref to packed struct's field
    // may not be properly aligned.
    // See https://github.com/rust-lang/rust/issues/82523
    pub fn set_raw_handler(&mut self, entry: Exception, handler: RawHandlerFunc) -> &mut Entry {
        self.set_entry(entry, handler as usize)
    }

    pub fn set_raw_handler_with_error_code(
        &mut self,
        entry: Exception,
        handler: RawHandlerFuncWithErrorCode,
    ) -> &mut Entry {
        self.set_entry(entry, handler as usize)
    }

    pub fn set_handler(&mut self, entry: Exception, handler: HandlerFunc) -> &mut Entry {
        self.set_entry(entry, handler as usize)
    }

    pub fn set_handler_with_error_code(
        &mut self,
        entry: Exception,
        handler: HandlerFuncWithErrorCode,
    ) -> &mut Entry {
        self.set_entry(entry, handler as usize)
    }

    fn set_entry(&mut self, entry: Exception, handler: usize) -> &mut Entry {
        let e = &mut self.0[entry as usize];
        *e = Entry::new(CS::get_reg(), handler);
        e
    }

    pub fn load(&'static self) {
        let ptr = DescriptorTablePointer {
            limit: (core::mem::size_of::<Idt>() - 1) as u16,
            base: VirtAddr(self as *const Idt as u64),
        };
        // SAFETY:
        // * The handler is valid idt and of 'static.
        unsafe {
            lidt(&ptr);
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Entry {
    pointer_low: u16,
    gdt_selector: SegmentSelector,
    options: EntryOptions,
    pointer_middle: u16,
    pointer_high: u32,
    reserved: u32,
}

impl Entry {
    fn new(gdt_selector: SegmentSelector, handler: usize) -> Self {
        let pointer = handler as u64;
        Entry {
            pointer_low: pointer as u16,
            gdt_selector,
            options: EntryOptions::new(),
            pointer_middle: (pointer >> 16) as u16,
            pointer_high: (pointer >> 32) as u32,
            reserved: 0,
        }
    }

    fn missing() -> Self {
        Entry {
            pointer_low: 0,
            gdt_selector: SegmentSelector::NULL,
            options: EntryOptions::minimal(),
            pointer_middle: 0,
            pointer_high: 0,
            reserved: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct EntryOptions(u16);

impl EntryOptions {
    fn minimal() -> Self {
        let mut options = 0;
        options.set_bits(9..=11, 0b111);
        EntryOptions(options)
    }

    fn new() -> Self {
        let mut options = Self::minimal();
        options.set_present(true).disable_interrupts(true);
        options
    }

    pub fn set_present(&mut self, present: bool) -> &mut Self {
        self.0.set_bits(15, present as u16);
        self
    }

    pub fn disable_interrupts(&mut self, disable: bool) -> &mut Self {
        self.0.set_bits(8, !disable as u16);
        self
    }

    pub fn set_privilege_level(&mut self, dpl: u16) -> &mut Self {
        self.0.set_bits(13..=14, dpl);
        self
    }

    pub fn set_stack_index(&mut self, index: u16) -> &mut Self {
        self.0.set_bits(0..=2, index);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serial_println;
    use crate::x86_64;

    #[test_case]
    fn test_breakpoint_handler() {
        init_idt();
        serial_println!("go!");
        x86_64::int3();
        crate::println!("haoye!");
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
    //     init_idt();
    //     serial_println!("go!");
    //     unsafe {
    //         *(0xdeadbeef as *mut u8) = 42;
    //     }
    //     serial_println!("No haoye!");
    // }

    fn overflow() {
        overflow();
    }

    #[test_case]
    fn test_stackoverflow() {
        overflow();
    }
}
