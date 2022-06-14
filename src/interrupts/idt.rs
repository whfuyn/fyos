use crate::bit_field::BitField;
use crate::lazy_static;
use crate::spinlock::SpinLock;
use crate::x86_64::*;
use crate::serial_println;
use core::arch::asm;

// lazy_static! {
//     pub static ref IDT: SpinLock<Idt> = SpinLock::new(Idt::new());
//     pub static ref DTP: DescriptorTablePointer = {
//         let mut idt = IDT.lock();
//         idt.set_handler(0, capture_divid_by_zero);
//         DescriptorTablePointer {
//             limit: (core::mem::size_of::<Idt>() - 1) as u16,
//             base: VirtAddr((&*idt) as *const Idt as u64),
//         }
//     };
// }

lazy_static! {
    static ref IDT: Idt = {
        let mut idt = Idt::new();
        idt.set_handler_raw(0, divide_by_zero_wrapper);
        idt.set_handler(3, breakpoint_handler);
        idt
    };
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    serial_println!("Haoye! It's a breakpoint!");
    serial_println!("StackFrame:\n{:#?}", stack_frame);
}

#[naked]
extern "C" fn divide_by_zero_wrapper() -> ! {
    // TODO: how to specify clobber rdi? Is this asm correct?
    // SAFETY:
    // * Called as a interrupt handler
    unsafe {
        asm!(
            "mov rdi, rsp",
            // Align the stack pointer
            "sub rsp, 8",
            "call {}",
            // Did I do it right?
            // TODO: add an const item to static assert that the signature is valid
            sym divid_by_zero_handler,
            options(noreturn)
        )
    }
}

extern "C" fn divid_by_zero_handler(stack_frame: &InterruptStackFrame) -> ! {
    crate::serial_println!("looks at the stack frame!");
    crate::serial_println!("{:#?}", stack_frame);
    loop {}
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
    pub fn set_handler_raw(&mut self, entry: u8, handler: RawHandlerFunc) -> &mut Entry {
        let e = &mut self.0[entry as usize];
        *e = Entry::new(CS::get_reg(), handler);
        e
    }

    pub fn set_handler(&mut self, entry: u8, handler: HandlerFunc) -> &mut Entry {
        let e = &mut self.0[entry as usize];
        *e = Entry::new_(CS::get_reg(), handler);
        e
    }

    pub fn load(&'static self) {
        let ptr = DescriptorTablePointer {
            limit: (core::mem::size_of::<Idt>() - 1) as u16,
            base: VirtAddr(self as *const Idt as u64),
        };
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
    fn new_(gdt_selector: SegmentSelector, handler: HandlerFunc) -> Self {
        #[allow(clippy::fn_to_numeric_cast)]
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

    fn new(gdt_selector: SegmentSelector, handler: RawHandlerFunc) -> Self {
        #[allow(clippy::fn_to_numeric_cast)]
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
    use crate::x86_64;
    use crate::serial_println;

    #[test_case]
    fn test_breakpoint_handler() {
        init_idt();
        serial_println!("go!");
        x86_64::int3();
        serial_println!("haoye!");
    }

    #[test_case]
    fn test_divid_by_zero_handler() {
        init_idt();
        serial_println!("go!");
        x86_64::divide_by_zero();
        serial_println!("ok!");
    }
}
