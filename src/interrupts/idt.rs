pub struct Idt([Entry; 16]);

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Entry {
    pointer_low: u16,
    // gdt_selector: SegmentSelector,
    options: EntryOptions,
    pointer_middle: u16,
    pointer_high: u32,
    reserved: u32,
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct EntryOptions(u16);

impl EntryOptions {
    fn new() -> Self {
        todo!()
    }

    pub fn set_present(&mut self) {
        todo!()
    }

    pub fn disable_interrupts(&mut self, disable: bool) {
        todo!()
    }

    pub fn set_privilege_level(&mut self, dpl: u16) {
        todo!()
    }

    pub fn set_stack_index(&mut self, index: u16) {
        self.0 = (self.0 & (!0b111)) | index;
    }
}
