#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]
#![feature(format_args_nl)]

mod lazy_static;
mod screen;
mod spinlock;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

static HELLO: &str = "Hello World!";
static MORNING: &str = "Morning! Nice day for fishing ain't it!";

#[derive(Debug)]
enum Error {
    FmtError(core::fmt::Error),
}

impl From<core::fmt::Error> for Error {
    fn from(e: core::fmt::Error) -> Self {
        Self::FmtError(e)
    }
}

fn main() -> Result<(), Error> {
    println!("{}\n", HELLO)?;
    println!("{}\n", MORNING)?;
    print!("{}", MORNING)?;
    Ok(())
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    main().unwrap();

    loop {
        core::hint::spin_loop();
    }
}
