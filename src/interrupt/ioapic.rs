//! IOAPIC.

use core::mem::MaybeUninit;

use x86::apic::{ioapic::IoApic, ApicControl};

pub static mut IOAPIC: MaybeUninit<IoApic> = MaybeUninit::zeroed();

pub unsafe fn init(ioapic_base: usize) {
    let mut ioapic = IoApic::new(ioapic_base);
    // IOAPIC.write(ioapic);
    unsafe { (*core::ptr::addr_of_mut!(IOAPIC)).write(ioapic); }

}

pub unsafe fn init_cpu() {
    let mut cpu = crate::cpu::get_current();

    // let ioapic = IOAPIC.assume_init_mut();
    let ioapic = unsafe { &mut *core::ptr::addr_of_mut!(IOAPIC) }.assume_init_mut();

    ioapic.enable(0, crate::cpu::get_cpu_id() as u8);
    ioapic.enable(1, crate::cpu::get_cpu_id() as u8);
}
