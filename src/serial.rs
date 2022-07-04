use uart_16550::SerialPort;

use crate::lazy_static;
use crate::spinlock::SpinLock;

lazy_static! {
    pub static ref SERIAL1: SpinLock<SerialPort> = {
        let mut serial_port = unsafe { SerialPort::new(0x3f8) };
        serial_port.init();
        SpinLock::new(serial_port)
    };
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    crate::interrupts::without_interrupts(
        || SERIAL1.lock().write_fmt(args).unwrap()
    );
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(::core::format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! serial_println {
    () => {
        $crate::serial_print!("\n");
    };
    ($($arg:tt)*) => {
        $crate::serial_print!("{}\n", ::core::format_args!($($arg)*));
    };
}
