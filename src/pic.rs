/// See pic8259

use crate::port::{
    access, Port, PortWrite, PortRead,
};

/// Command sent to begin PIC initialization.
const CMD_INIT: u8 = 0x11;
/// Command sent to acknowledge an interrupt.
const CMD_END_OF_INTERRUPT: u8 = 0x20;
/// The mode in which we want to run our PICSs.
const MODE_8086: u8 = 0x01;

struct Pic {
    offset: u8,
    cmd: Port<u8>,
    data: Port<u8>,
}

impl Pic {
    fn end_of_interrupt(&mut self) {
        self.cmd.write(CMD_END_OF_INTERRUPT);
    }

    fn read_mask(&self) -> u8 {
        // TODO: why can we just read the data port for the mask?
        self.data.read()
    }

    fn write_mask(&mut self, mask: u8) {
        self.data.write(mask)
    }

    fn handles_interrupt(&self, interrupt_id: u8) -> bool {
        self.offset <= interrupt_id && interrupt_id < self.offset + 8
    }
}

pub struct ChainedPics {
    pics: [Pic; 2]
}

impl ChainedPics {
    /// Safety(?):
    /// * Must not overlap with exception.
    /// * Must not overlap with each other.
    pub const unsafe fn new(offset1: u8, offset2: u8) -> Self {
        ChainedPics {
            pics: [
                Pic {
                    offset: offset1,
                    cmd: Port::new(0x20),
                    data: Port::new(0x21),
                },
                Pic {
                    offset: offset2,
                    cmd: Port::new(0xa0),
                    data: Port::new(0xa1),
                },
            ]
        }
    }

    fn read_masks(&self) -> [u8; 2] {
        [self.pics[0].read_mask(), self.pics[1].read_mask()]
    }

    fn write_masks(&mut self, mask1: u8, mask2: u8) {
        self.pics[0].write_mask(mask1);
        self.pics[1].write_mask(mask2);
    }

    pub unsafe fn initialize(&mut self) {
        let mut wait_port: Port<u8> = Port::new(0x80);
        let mut wait = || wait_port.write(0);

        let saved_mask = self.read_masks();

        // Tell each PIC that we're going to send it a three-byte
        // initialization sequence on its data port.
        self.pics[0].cmd.write(CMD_INIT);
        wait();
        self.pics[1].cmd.write(CMD_INIT);
        wait();

        // Byte 1: Set up base offset
        self.pics[0].data.write(self.pics[0].offset);
        wait();
        self.pics[1].data.write(self.pics[1].offset);
        wait();

        // Byte 2: Confiture chaining between PIC1 and PIC2
        self.pics[0].data.write(4);
        wait();
        self.pics[1].data.write(2);
        wait();

        // Byte 3: Set out mode
        self.pics[0].data.write(MODE_8086);
        wait();
        self.pics[1].data.write(MODE_8086);
        wait();

        self.write_masks(saved_mask[0], saved_mask[1]);
        // There is no more waiting after write_masks in pic8259 crate.
        // But why? I gonna add it anyway.
        wait();
    }

    pub fn handles_interrupt(&self, interrupt_id: u8) -> bool {
        self.pics.iter().any(|p| p.handles_interrupt(interrupt_id))
    }

    pub unsafe fn disable(&mut self) {
        self.write_masks(u8::MAX, u8::MAX);
    }

    pub fn notify_end_of_interrupt(&mut self, interrupt_id: u8) {
        if self.handles_interrupt(interrupt_id) {
            if self.pics[1].handles_interrupt(interrupt_id) {
                self.pics[1].end_of_interrupt();
            }
            self.pics[0].end_of_interrupt();
        }
    }

}
