//! LAPIC.
//!
//! We just use the xAPIC implementation in the x86 crate.

use core::arch::asm;
use core::mem::MaybeUninit;
use core::slice;

use super::x86_xapic::XAPIC;
use x86::apic::{ApicControl, ApicId};
use x86::msr;

use super::Cycles;
// use crate::{boot, cpu};

use crate::cpu::{self, get_cpu_id};
//use crate::cpu;
/// Returns the 4KiB LAPIC region.
unsafe fn probe_apic() -> &'static mut [u32] {
    let msr27: u32 = msr::rdmsr(msr::APIC_BASE) as u32;
    let lapic = (msr27 & 0xffff_0000) as usize as *mut u32;
    slice::from_raw_parts_mut(lapic, 4096 / 4)
}

/// Initializes LAPIC in xAPIC mode.
pub unsafe fn init() {
    let cpu = cpu::get_current();
    let apic_region: &'static mut [u32] = probe_apic();
    let mut xapic = XAPIC::new(apic_region);
    xapic.attach();
    xapic.tsc_set_oneshot(0xfffffffe); 
    xapic.tsc_enable(32);

    cpu.xapic.write(xapic);
}

/// Arms the timer interrupt.
pub fn set_timer(cycles: Cycles) {
    let xapic = unsafe {
        crate::cpu::get_current().xapic.assume_init_mut()
        //(&mut *crate::cpu::get_current_cpu_field_ptr!(xapic, MaybeUninit<XAPIC>)).assume_init_mut()
    };

    // FIXME: Truncated
    xapic.tsc_set_oneshot(cycles.0 as u32);
}

/// Acknowledges an interrupt.
pub fn end_of_interrupt() {
    let xapic = unsafe {
        crate::cpu::get_current().xapic.assume_init_mut()
        // (&mut *crate::cpu::get_current_cpu_field_ptr!(xapic, MaybeUninit<XAPIC>)).assume_init_mut()
    };

    xapic.eoi();
}

/// Boots an application processor.
pub unsafe fn boot_ap(cpu_id: u32, stack: u64, code: u64) {
    // Will need to implement this to boot other CPUs, but not now    
}
