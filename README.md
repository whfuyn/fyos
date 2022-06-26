# fyos

A hobbyist operating system written in Rust. 

See [Writing an OS in Rust](https://os.phil-opp.com/).

I use this tutorial as a guide. Things of interest will be implemented on my own.


## Prerequisites

- `rust-src` is needed to build our kernel for a custom target.
- To create a bootable image, install `bootimage` binary and its denpendency `llvm-tools-preview`.
```
$ rustup component rust-src llvm-tools-preview
$ cargo install bootimage
```

To run the kernel in QEMU, you need to install it yourself.

For Arch Linux:
```
$ pacman -S qemu-full
```


## Usage
```
$ cargo run
```

This will run the built kernel in QEMU.
