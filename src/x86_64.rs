// See https://docs.rs/x86_64

use core::arch::asm;
use core::fmt;
use core::ops;

#[repr(u8)]
pub enum PrivilegeLevel {
    Ring0 = 0,
    Ring1 = 1,
    Ring2 = 2,
    Ring3 = 3,
}

pub struct CS;

impl CS {
    /// Safety:
    /// * input must be valid for cs.
    pub unsafe fn set_reg(cs: SegmentSelector) {
        unsafe {
            asm!(
                "push {sel}",
                // 1f means label 1 searched forward
                "lea {tmp}, [1f + rip]",
                "push {tmp}",
                "retfq",
                "1:",
                sel = in(reg) u64::from(cs.0),
                tmp = lateout(reg) _,
                options(preserves_flags),
            );
        }
    }

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct VirtAddr(pub u64);

impl VirtAddr {
    pub const fn zero() -> Self {
        VirtAddr(0)
    }

    pub fn from_ptr<T>(ptr: *const T) -> Self {
        VirtAddr(ptr as u64)
    }
}

// TODO: consider those ops impls.
impl ops::Add<u64> for VirtAddr {
    type Output = Self;

    fn add(self, rhs: u64) -> Self::Output {
        VirtAddr(self.0.checked_add(rhs).unwrap())
    }
}

impl ops::Add<usize> for VirtAddr {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        VirtAddr(self.0.checked_add(rhs as u64).unwrap())
    }
}

impl fmt::LowerHex for VirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

impl fmt::UpperHex for VirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::UpperHex::fmt(&self.0, f)
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed(2))]
pub struct DescriptorTablePointer {
    pub limit: u16,
    pub base: VirtAddr,
}

/// Load interrupt descriptor table
/// Safety:
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

/// Load global descriptor table
/// Safety:
/// * GDT is valid & 'static
/// * There may be other requirements I don't know.
#[inline]
pub unsafe fn lgdt(gdt: &DescriptorTablePointer) {
    unsafe {
        asm!(
            "lgdt [{}]",
            in(reg) gdt,
            options(readonly, nostack, preserves_flags)
        );
    }
}

#[inline]
pub fn int3() {
    unsafe {
        asm!("int 3");
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

#[inline]
pub fn ud2() {
    unsafe {
        asm!("ud2");
    }
}

#[inline]
pub fn hlt() {
    unsafe {
        asm!(
            "hlt",
            options(nomem, nostack, preserves_flags),
        )
    }
}

#[inline]
pub fn hlt_loop() -> ! {
    loop {
        hlt();
    }
}

#[inline]
pub fn enable_interrupt() {
    unsafe {
        asm!(
            "sti",
            options(nomem, nostack)
        )
    }
}

#[inline]
pub fn disable_interrupt() {
    unsafe {
        asm!(
            "cli",
            options(nomem, nostack)
        )
    }
}

pub fn is_interrupt_enabled() -> bool {
    const INTERRUPT_FLAG: u64 = 1 << 9;
    let rflags: u64;
    unsafe {
        asm!(
            "pushfq",
            "pop {}",
            out(reg) rflags,
            options(nomem, preserves_flags)
        );
    }
    rflags & INTERRUPT_FLAG != 0
}

pub fn without_interrupts<F: FnOnce() -> R, R>(f: F) -> R {
    // TODO: will it cause a race condition where interrupt state changed
    // during the process?
    let is_enabled = is_interrupt_enabled();
    if is_enabled {
        disable_interrupt();
        let ret = f();
        enable_interrupt();
        ret
    } else {
        f()
    }
}

/// Safety:
/// * input is an valid tss
pub unsafe fn load_tss(tss: SegmentSelector) {
    unsafe {
        asm!(
            "ltr {:x}", in(reg) tss.0, options(nostack, preserves_flags)
        )
    }
}
