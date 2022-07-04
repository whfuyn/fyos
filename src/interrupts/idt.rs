use core::marker::PhantomData;
use core::ops::{ Index, IndexMut };
use crate::bit_field::BitField;
use crate::gdt;
use crate::lazy_static;
use crate::raw_handler;
use crate::raw_page_fault_handler;
use crate::raw_handler_with_error_code;
use crate::print;
use crate::serial_print;
use crate::serial_println;
use crate::x86_64::{
    lidt, DescriptorTablePointer,
    SegmentSelector, VirtAddr, CS,
};
use super::{
    ErrorCode, PageFaultErrorCode, InterruptStackFrame,
    HandlerFunc, HandlerFuncWithErrorCode, PageFaultHandlerFunc, 
    DivergingHandlerFunc, DivergingHandlerFuncWithErrorCode,
    RawHandlerFunc, RawHandlerFuncWithErrorCode, RawPageFaultHandlerFunc,
    RawDivergingHandlerFunc, RawDivergingHandlerFuncWithErrorCode,
    HandlerFn,
};

#[derive(Clone)]
#[repr(C)]
#[repr(align(16))]
pub struct InterruptDescriptorTable {
    pub divide_error: Entry<HandlerFunc>,
    pub debug: Entry<HandlerFunc>,
    pub non_maskable_interrupt: Entry<HandlerFunc>,
    pub breakpoint: Entry<HandlerFunc>,
    pub overflow: Entry<HandlerFunc>,
    pub bound_range_exceeded: Entry<HandlerFunc>,
    pub invalid_opcode: Entry<HandlerFunc>,
    pub device_not_available: Entry<HandlerFunc>,
    pub double_fault: Entry<DivergingHandlerFuncWithErrorCode>,
    coprocessor_segment_overrun: Entry<HandlerFunc>,
    pub invalid_tss: Entry<HandlerFuncWithErrorCode>,
    pub segment_not_present: Entry<HandlerFuncWithErrorCode>,
    pub stack_segment_fault: Entry<HandlerFuncWithErrorCode>,
    pub general_protection_fault: Entry<HandlerFuncWithErrorCode>,
    pub page_fault: Entry<PageFaultHandlerFunc>,
    /// vector nr. 15
    reserved_1: Entry<HandlerFunc>,
    pub x87_floating_point: Entry<HandlerFunc>,
    pub alignment_check: Entry<HandlerFuncWithErrorCode>,
    pub machine_check: Entry<DivergingHandlerFunc>,
    pub simd_floating_point: Entry<HandlerFunc>,
    pub virtualization: Entry<HandlerFunc>,
    /// vector nr. 21-28
    reserved_2: [Entry<HandlerFunc>; 8],
    pub vmm_communication_exception: Entry<HandlerFuncWithErrorCode>,
    pub security_exception: Entry<HandlerFuncWithErrorCode>,
    /// vector nr. 31
    reserved_3: Entry<HandlerFunc>,
    interrupts: [Entry<HandlerFunc>; 256 - 32],
}

impl InterruptDescriptorTable {
    pub fn new() -> InterruptDescriptorTable {
        InterruptDescriptorTable {
            divide_error: Entry::missing(),
            debug: Entry::missing(),
            non_maskable_interrupt: Entry::missing(),
            breakpoint: Entry::missing(),
            overflow: Entry::missing(),
            bound_range_exceeded: Entry::missing(),
            invalid_opcode: Entry::missing(),
            device_not_available: Entry::missing(),
            double_fault: Entry::missing(),
            coprocessor_segment_overrun: Entry::missing(),
            invalid_tss: Entry::missing(),
            segment_not_present: Entry::missing(),
            stack_segment_fault: Entry::missing(),
            general_protection_fault: Entry::missing(),
            page_fault: Entry::missing(),
            reserved_1: Entry::missing(),
            x87_floating_point: Entry::missing(),
            alignment_check: Entry::missing(),
            machine_check: Entry::missing(),
            simd_floating_point: Entry::missing(),
            virtualization: Entry::missing(),
            reserved_2: [Entry::missing(); 8],
            vmm_communication_exception: Entry::missing(),
            security_exception: Entry::missing(),
            reserved_3: Entry::missing(),
            interrupts: [Entry::missing(); 256 - 32],
        }
    }

    pub fn load(&'static self) {
        let ptr = DescriptorTablePointer {
            limit: (core::mem::size_of::<Self>() - 1) as u16,
            base: VirtAddr(self as *const Self as u64),
        };
        // Safety:
        // * The handler is valid idt and of 'static.
        unsafe {
            lidt(&ptr);
        }
    }
}

impl Index<usize> for InterruptDescriptorTable {
    type Output = Entry<HandlerFunc>;

    /// Returns the IDT entry with the specified index.
    ///
    /// Panics if index is outside the IDT (i.e. greater than 255) or if the entry is an
    /// exception that pushes an error code (use the struct fields for accessing these entries).
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.divide_error,
            1 => &self.debug,
            2 => &self.non_maskable_interrupt,
            3 => &self.breakpoint,
            4 => &self.overflow,
            5 => &self.bound_range_exceeded,
            6 => &self.invalid_opcode,
            7 => &self.device_not_available,
            9 => &self.coprocessor_segment_overrun,
            16 => &self.x87_floating_point,
            19 => &self.simd_floating_point,
            20 => &self.virtualization,
            i @ 32..=255 => &self.interrupts[i - 32],
            i @ 15 | i @ 31 | i @ 21..=28 => panic!("entry {} is reserved", i),
            i @ 8 | i @ 10..=14 | i @ 17 | i @ 29 | i @ 30 => {
                panic!("entry {} is an exception with error code", i)
            }
            i @ 18 => panic!("entry {} is an diverging exception (must not return)", i),
            i => panic!("no entry with index {}", i),
        }
    }
}

impl IndexMut<usize> for InterruptDescriptorTable {
    /// Returns a mutable reference to the IDT entry with the specified index.
    ///
    /// Panics if index is outside the IDT (i.e. greater than 255) or if the entry is an
    /// exception that pushes an error code (use the struct fields for accessing these entries).
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0 => &mut self.divide_error,
            1 => &mut self.debug,
            2 => &mut self.non_maskable_interrupt,
            3 => &mut self.breakpoint,
            4 => &mut self.overflow,
            5 => &mut self.bound_range_exceeded,
            6 => &mut self.invalid_opcode,
            7 => &mut self.device_not_available,
            9 => &mut self.coprocessor_segment_overrun,
            16 => &mut self.x87_floating_point,
            19 => &mut self.simd_floating_point,
            20 => &mut self.virtualization,
            i @ 32..=255 => &mut self.interrupts[i - 32],
            i @ 15 | i @ 31 | i @ 21..=28 => panic!("entry {} is reserved", i),
            i @ 8 | i @ 10..=14 | i @ 17 | i @ 29 | i @ 30 => {
                panic!("entry {} is an exception with error code", i)
            }
            i @ 18 => panic!("entry {} is an diverging exception (must not return)", i),
            i => panic!("no entry with index {}", i),
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Entry<F: HandlerFn> {
    pointer_low: u16,
    gdt_selector: SegmentSelector,
    options: EntryOptions,
    pointer_middle: u16,
    pointer_high: u32,
    reserved: u32,
    // handler type
    _phantom_handler: PhantomData<F>,
}

impl<F: HandlerFn> Entry<F> {
    unsafe fn set_handler_addr(&mut self, addr: VirtAddr) -> &mut EntryOptions {
        let pointer = addr.0;
        self.pointer_low = pointer as u16;
        self.pointer_middle = (pointer >> 16) as u16;
        self.pointer_high = (pointer >> 32) as u32;
        self.gdt_selector = CS::get_reg();
        self.options.set_present(true);
        &mut self.options
    }

    fn missing() -> Self {
        Entry {
            pointer_low: 0,
            gdt_selector: SegmentSelector::NULL,
            options: EntryOptions::minimal(),
            pointer_middle: 0,
            pointer_high: 0,
            reserved: 0,
            _phantom_handler: PhantomData,
        }
    }
}

macro_rules! impl_set_handler {
    ($handler_ty:ty) => {
        impl Entry<$handler_ty> {
            pub fn set_handler(&mut self, handler: <$handler_ty as $crate::interrupts::HandlerFn>::Handler) -> &mut EntryOptions {
                unsafe {
                    self.set_handler_addr(VirtAddr(handler as u64))
                }
            }

            pub fn set_raw_handler(
                &mut self, handler:
                    $crate::interrupts::RawHandler<
                        <$handler_ty as $crate::interrupts::HandlerFn>::RawHandler
                    >
            ) -> &mut EntryOptions {
                unsafe {
                    self.set_handler_addr(VirtAddr(handler.handler as u64))
                }
            }
        }
    };
    ($handler_ty:ty, $($rest:tt)*) => {
        impl_set_handler!($handler_ty);
        impl_set_handler!($($rest)*);
    };
    () => {};
}

impl_set_handler!{
    HandlerFunc,
    DivergingHandlerFunc,
    HandlerFuncWithErrorCode,
    DivergingHandlerFuncWithErrorCode,
    PageFaultHandlerFunc,
    RawHandlerFunc,
    RawDivergingHandlerFunc,
    RawHandlerFuncWithErrorCode,
    RawDivergingHandlerFuncWithErrorCode,
    RawPageFaultHandlerFunc,
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

    /// Safety:
    /// * stack index is a valid and not used by other interrupts.
    pub unsafe fn set_stack_index(&mut self, index: u16) -> &mut Self {
        // The hardware IST index starts at 1, but our software IST index
        // starts at 0. Therefore we need to add 1 here.
        self.0.set_bits(0..=2, index + 1);
        self
    }
}
