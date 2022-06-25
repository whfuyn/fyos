/// See x86_64::instructions::port
use core::arch::asm;
use core::marker::PhantomData;

pub mod access {
    pub trait Readable {}
    pub trait Writable {}

    pub struct PortReadWrite;

    impl Readable for PortReadWrite {}
    impl Writable for PortReadWrite {}

    pub struct PortReadOnly;
    impl Readable for PortReadOnly {}

    pub struct PortWriteOnly;
    impl Writable for PortWriteOnly {}
}

/// T is the byte length(u8, u16, u32) we read from or write to this port.
/// A is the access permission of the port.
pub struct PortGeneric<T, A> {
    port: u16,
    _phantom: PhantomData<(T, A)>,
}

pub trait Write<T> {
    fn write(&self, value: T);
}

pub trait Read<T> {
    fn read(&self) -> T;
}

impl<A: access::Writable> Write<u8> for PortGeneric<u8, A> {
    fn write(&self, value: u8) {
        // See https://www.felixcloutier.com/x86/out
        // SAFETY:
        // TODO: in what circumstance will it be unsound?
        unsafe {
            asm!(
                "out dx, al",
                in("dx") self.port,
                in("al") value,
                options(nostack, nomem, preserves_flags)
            )
        }
    }
}

impl<A: access::Writable> Write<u16> for PortGeneric<u16, A> {
    fn write(&self, value: u16) {
        unsafe {
            asm!(
                "out dx, ax",
                in("dx") self.port,
                in("ax") value,
                options(nostack, nomem, preserves_flags)
            )
        }
    }
}

impl<A: access::Writable> Write<u32> for PortGeneric<u32, A> {
    fn write(&self, value: u32) {
        unsafe {
            asm!(
                "out dx, eax",
                in("dx") self.port,
                in("eax") value,
                options(nostack, nomem, preserves_flags)
            )
        }
    }
}

impl<A: access::Readable> Read<u8> for PortGeneric<u8, A> {
    fn read(&self) -> u8 {
        // Looks like both `let mut` and `let` will work.
        let value: u8;
        unsafe {
            asm!(
                "in al, dx",
                in("dx") self.port,
                out("al") value,
                options(nostack, nomem, preserves_flags)
            );
        }
        value
    }
}

impl<A: access::Readable> Read<u16> for PortGeneric<u16, A> {
    fn read(&self) -> u16 {
        let value: u16;
        unsafe {
            asm!(
                "in ax, dx",
                in("dx") self.port,
                out("ax") value,
                options(nostack, nomem, preserves_flags)
            );
        }
        value
    }
}

impl<A: access::Readable> Read<u32> for PortGeneric<u32, A> {
    fn read(&self) -> u32 {
        let value: u32;
        unsafe {
            asm!(
                "in eax, dx",
                in("dx") self.port,
                out("eax") value,
                options(nostack, nomem, preserves_flags)
            );
        }
        value
    }
}
