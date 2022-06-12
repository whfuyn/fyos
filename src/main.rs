#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]

mod lazy_static;
mod screen;
mod spinlock;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{}", info);
    loop {
        core::hint::spin_loop();
    }
}

static HELLO: &str = "Hello World!";
static MORNING: &str = "Morning! Nice day for fishing ain't it!";

fn main() {
    println!("{}\n", HELLO);
    for i in 1.. {
        println!("{} - {}", MORNING, i);
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    main();

    loop {
        core::hint::spin_loop();
    }
}
