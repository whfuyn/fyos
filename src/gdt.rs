use crate::lazy_static;
use crate::x86_64::{PrivilegeLevel, SegmentSelector, VirtAddr};

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

    static ref GDT: GlobalDescriptorTable = {
        let gdt = GlobalDescriptorTable::new();
        todo!()

    };
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed(4))]
struct TaskStateSegment {
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
            iomap_base: core::mem::size_of::<TaskStateSegment>() as u16,
            reserved_1: 0,
            reserved_2: 0,
            reserved_3: 0,
            reserved_4: 0,
        }
    }
}

#[derive(Debug, Clone)]
struct GlobalDescriptorTable {
    table: [u64; 8],
    len: usize,
}

impl GlobalDescriptorTable {
    #[inline]
    pub const fn new() -> Self {
        // TODO: why is len 1?
        Self {
            table: [0; 8],
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

    fn push(&mut self, value: u64) -> usize {
        let index = self.len;
        self.table[self.len] = value;
        self.len += 1;
        index
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
    pub const USER_SEGMENT: u64 = 1 << 44;
    pub const DPL_RING_3: u64 = 3 << 45;
    pub const PRESENT: u64 = 1 << 47;

    pub const COMMON: u64 = Self::USER_SEGMENT | Self::ACCESSED | Self::PRESENT;

    pub const KERNEL_CODE64: u64 = Self::COMMON;
}

impl Descriptor {
    fn kernel_segment() -> Self {
        Descriptor::UserSegment(DescriptorFlags::KERNEL_CODE64)
    }

    fn tss_segment(tss: &'static TaskStateSegment) -> Self {
        let ptr = tss as *const _ as u64;
        todo!()
    }
}
