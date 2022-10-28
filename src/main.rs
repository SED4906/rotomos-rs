#![no_std]
#![no_main]
#![feature(strict_provenance)]
#![feature(error_in_core)]

pub mod writer;
pub mod memory;
pub mod task;

use core::panic::PanicInfo;
use limine::*;
use crate::memory::build_freelist;

static TERMINAL_REQUEST: LimineTerminalRequest = LimineTerminalRequest::new(0);
static BOOTLOADER_INFO: LimineBootInfoRequest = LimineBootInfoRequest::new(0);
static MMAP: LimineMmapRequest = LimineMmapRequest::new(0);

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    if let Some(s) = _info.payload().downcast_ref::<&str>() {
        println!("panic: {s:?}");
    } else {
        println!("panic: ??");
    }
    loop {}
}

// define the kernel's entry point function
#[no_mangle]
extern "C" fn x86_64_barebones_main() -> ! {
    println!("Rotom rotom!\n");

    let bootloader_info = BOOTLOADER_INFO
        .get_response()
        .get()
        .expect("rotom: recieved no bootloader info");

    println!(
        "bootloader: (name={:?}, version={:?})",
        bootloader_info.name.to_string().unwrap(),
        bootloader_info.version.to_string().unwrap()
    );

    let mmap = MMAP
        .get_response()
        .get()
        .expect("rotom: recieved no mmap")
        .mmap();

    //println!("mmap: {:#x?}", mmap);

    build_freelist(mmap);

    loop {}
}