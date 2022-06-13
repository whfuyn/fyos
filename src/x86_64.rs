// See https://docs.rs/x86_64

use core::arch::asm;

pub type HandlerFunc = extern "C" fn() -> !;

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
