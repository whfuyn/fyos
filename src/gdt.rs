use crate::lazy_static;
use crate::x86_64::{
    lgdt, load_tss, DescriptorTablePointer, PrivilegeLevel, SegmentSelector, VirtAddr, CS,
};
use core::mem::size_of;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(unsafe { &STACK });
            // Notice the property of x86 stack, i.e. grows downward
            stack_start + STACK_SIZE
        };
        tss
    };

    // Due to the implementation, we cannot define multiple statics like
    // static ref (GDT, CODE_SELECTOR, TSS_SELECTOR) = {..};
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        (gdt, Selectors{code_selector, tss_selector})
    };
}

struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

pub fn init() {
    GDT.0.load();
    // SAFETY: code and tss selector are valid.
    unsafe {
        CS::set_reg(GDT.1.code_selector);
        load_tss(GDT.1.tss_selector);
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed(4))]
pub struct TaskStateSegment {
    reserved_1: u32,
    /// The full 64-bit canonical forms of the stack pointers (RSP) for privilege levels 0-2.
    pub privilege_stack_table: [VirtAddr; 3],
    reserved_2: u64,
    /// The full 64-bit canonical forms of the interrupt stack table (IST) pointers.
    pub interrupt_stack_table: [VirtAddr; 7],
    reserved_3: u64,
    reserved_4: u16,
    /// The 16-bit offset to the I/O permission bit map from the 64-bit TSS base.
    pub iomap_base: u16,
}

impl TaskStateSegment {
    // TODO: make sure we understand below comments.
    /// Creates a new TSS with zeroed privilege and interrupt stack table and an
    /// empty I/O-Permission Bitmap.
    ///
    /// As we always set the TSS segment limit to
    /// `size_of::<TaskStateSegment>() - 1`, this means that `iomap_base` is
    /// initialized to `size_of::<TaskStateSegment>()`.
    fn new() -> Self {
        Self {
            privilege_stack_table: [VirtAddr::zero(); 3],
            interrupt_stack_table: [VirtAddr::zero(); 7],
            iomap_base: size_of::<TaskStateSegment>() as u16,
            reserved_1: 0,
            reserved_2: 0,
            reserved_3: 0,
            reserved_4: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GlobalDescriptorTable {
    table: [u64; 8],
    len: usize,
}

impl GlobalDescriptorTable {
    #[inline]
    pub const fn new() -> Self {
        Self {
            table: [0; 8],
            // The first entry in the GDT (Entry 0) should always be null
            // and subsequent entries should be used instead.
            // See https://wiki.osdev.org/Global_Descriptor_Table#Table
            len: 1,
        }
    }

    pub fn add_entry(&mut self, entry: Descriptor) -> SegmentSelector {
        let index = match entry {
            Descriptor::UserSegment(value) => {
                if self.len > self.table.len().saturating_sub(1) {
                    panic!("GDT is full");
                }
                self.push(value)
            }
            Descriptor::SystemSegment(value_low, value_high) => {
                if self.len > self.table.len().saturating_sub(2) {
                    panic!("GDT requires two free spaces to hold a SystemSegment")
                }
                let index = self.push(value_low);
                self.push(value_high);
                index
            }
        };
        let rpl = match entry {
            Descriptor::UserSegment(value) => {
                if value & DescriptorFlags::DPL_RING_3 != 0 {
                    PrivilegeLevel::Ring3
                } else {
                    PrivilegeLevel::Ring0
                }
            }
            Descriptor::SystemSegment(_, _) => PrivilegeLevel::Ring0,
        };

        SegmentSelector::new(index as u16, rpl)
    }

    #[inline]
    fn push(&mut self, value: u64) -> usize {
        let index = self.len;
        self.table[index] = value;
        self.len += 1;
        index
    }

    fn pointer(&self) -> DescriptorTablePointer {
        DescriptorTablePointer {
            base: VirtAddr::from_ptr(self.table.as_ptr()),
            // The size of the table in bytes subtracted by 1. This subtraction
            // occurs because the maximum value of Size is 65535, while the GDT
            // can be up to 65536 bytes in length (8192 entries). Further, no GDT
            // can have a size of 0 bytes.
            // See https://wiki.osdev.org/Global_Descriptor_Table#GDTR
            limit: (size_of::<u64>() * self.len - 1) as u16,
        }
    }

    pub fn load(&'static self) {
        // SAFETY:
        // * valid & 'static
        unsafe { lgdt(&self.pointer()) }
    }
}

#[derive(Debug, Clone)]
pub enum Descriptor {
    UserSegment(u64),
    SystemSegment(u64, u64),
}

// TODO: refactor to bitflags after I write my own version.
struct DescriptorFlags;

impl DescriptorFlags {
    // Flags ignored in 64-bit mode are omitted.

    // TODO: What does below x86_64 comment mean?
    // * _Setting_ this bit in software prevents GDT writes on first use.
    pub const ACCESSED: u64 = 1 << 40;

    // --- It's said to be ignored in 64-bit
    pub const WRITABLE: u64 = 1 << 41;
    pub const GRANULARITY: u64 = 1 << 55;
    pub const LIMIT_0_15: u64 = 0xffff;
    pub const LIMIT_16_19: u64 = 0xf << 48;
    // ---

    pub const EXECUTABLE: u64 = 1 << 43;
    pub const USER_SEGMENT: u64 = 1 << 44;
    pub const DPL_RING_3: u64 = 3 << 45;
    pub const PRESENT: u64 = 1 << 47;
    pub const LONG_MODE: u64 = 1 << 53;

    pub const COMMON: u64 = Self::USER_SEGMENT
        | Self::PRESENT
        | Self::WRITABLE
        | Self::ACCESSED
        | Self::LIMIT_0_15
        | Self::LIMIT_16_19
        | Self::GRANULARITY;

    pub const KERNEL_CODE64: u64 = Self::COMMON | Self::LONG_MODE | Self::EXECUTABLE;
}

impl Descriptor {
    fn kernel_segment() -> Self {
        Descriptor::UserSegment(DescriptorFlags::KERNEL_CODE64)
    }

    fn tss_segment(tss: &'static TaskStateSegment) -> Self {
        use crate::bit_field::BitField;

        let ptr = tss as *const _ as u64;
        let mut low = DescriptorFlags::PRESENT;
        // base
        low.set_bits(16..40, ptr.get_bits(0..24));
        low.set_bits(56..64, ptr.get_bits(24..32));
        // TODO: what does those below comments mean?
        // limit (the `-1` in needed since the bound is inclusive)
        low.set_bits(0..16, (size_of::<TaskStateSegment>() - 1) as u64);
        // type (0b1001 = available 64-bit tss)
        low.set_bits(40..44, 0b1001);

        let mut high = 0;
        high.set_bits(0..32, ptr.get_bits(32..64));

        Descriptor::SystemSegment(low, high)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_flags() {
        assert_eq!(DescriptorFlags::KERNEL_CODE64, 0x00af9b000000ffffu64);
    }
}
