#![allow(warnings)]
#![cfg_attr(not(test), no_std, no_main)]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]

extern crate alloc;

use core::alloc::Layout;

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {
        $crate::serial_println!($($arg)*);
    };
}

#[alloc_error_handler]
fn alloc_error(layout: Layout) -> ! {
    panic!("kernel OOM: {:?}", layout);
}

use core::panic::PanicInfo;
mod serial;
mod error;
mod cpu;
mod gdt;
mod interrupt;
mod memory;
use crate::memory::{kernel_end};
mod multibootv2;

#[unsafe(no_mangle)]
pub extern "C" fn rust_main() -> ! {
    unsafe{
        // serial_println!("Hello from Rust!");
        crate::gdt::init_cpu();
        crate::interrupt::init();
        crate::interrupt::init_cpu();
        // crate::interrupt::set_timer(crate::interrupt::Cycles(1_000_000_000));
        unsafe extern "C" {
            #[unsafe(no_mangle)]
            static _bootinfo: usize;
        }
        let bootinfo = unsafe {
            multibootv2::load(_bootinfo)
        };

        unsafe {
            crate::memory::init_allocator(&bootinfo);
            crate::memory::test::test_all();
            // crate::memory::dump_allocator_header();
            // First few entries (should be Unavailable):
            // crate::memory::dump_page_array(0, 8);

            // Around the boundary (end_pfn - 4 .. end_pfn + 4):
            // crate::memory::dump_around_kernel_end(4);
            // let paddr = crate::memory::alloc_4k().expect("alloc_4k failed");
            // crate::memory::free_4k(paddr);
            // Optional (right now will be all Unavailable):
            // unsafe { crate::memory::test_alloc_free_basic(); }
            // crate::memory::dump_state_summary();
        }
        loop {}
    }

}

/// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_println!("Error! {}", _info);
    loop {}
}
