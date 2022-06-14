// See https://docs.rs/x86_64

use core::arch::asm;

pub type RawHandlerFunc = unsafe extern "C" fn() -> !;
pub type HandlerFunc = extern "x86-interrupt" fn(InterruptStackFrame);

#[repr(u8)]
pub enum PrivilegeLevel {
    Ring0 = 0,
    Ring1 = 1,
    Ring2 = 2,
    Ring3 = 3,
}

pub struct CS;

impl CS {
    pub fn get_reg() -> SegmentSelector {
        let mut cs: u16;
        unsafe {
            asm!(
                "mov {:x}, cs",
                out(reg) cs,
                options(nomem, nostack, preserves_flags)
            );
        }
        SegmentSelector(cs)
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct SegmentSelector(u16);

impl SegmentSelector {
    pub const NULL: Self = SegmentSelector::new(0, PrivilegeLevel::Ring0);

    /// rpl means Requested Privilege Level.
    pub const fn new(index: u16, rpl: PrivilegeLevel) -> Self {
        SegmentSelector(index << 3 | (rpl as u16))
    }
}

// TODO: impl Debug
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct VirtAddr(pub u64);

#[repr(C, packed(2))]
pub struct DescriptorTablePointer {
    pub limit: u16,
    pub base: VirtAddr,
}

/// Load interrupt descriptor table
/// SAFETY:
/// * Handler address must be valid.
/// * There may be other requirements I don't know.
#[inline]
pub unsafe fn lidt(idt: &DescriptorTablePointer) {
    unsafe {
        asm!(
            "lidt [{}]",
            in(reg) idt,
            options(readonly, nostack, preserves_flags)
        );
    }
}

#[inline]
pub fn int3() {
    unsafe {
        asm!(
            "int 3",
            options(preserves_flags)
        );
    }
}

#[inline]
pub fn divide_by_zero() {
    unsafe {
        asm!(
            "mov dx, 0",
            "div dx",
            out("dx") _,
            out("ax") _,
            options(nomem, nostack),
        );
    }
}

// /// SAFETY:
// /// * It's called in the begining of a raw interrupt handler.
// #[inline(always)]
// pub unsafe fn load_interrupt_stack_frame<'a>() -> &'a InterruptStackFrame {
//     let stack_frame: *const InterruptStackFrame;
//     unsafe {
//         asm!(
//             "mov {}, rsp",
//             out(reg) stack_frame,
//         );
//         &*stack_frame
//     }
// }

// TODO: impl dref and unsafe get_mut
/// Wrapper that ensures no accidental modification of the interrupt stack frame.(?)
#[derive(Debug)]
#[repr(C)]
pub struct InterruptStackFrame {
    value: InterruptStackFrameValue,
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

