//! The per-CPU data structure.
//!
//! The [`Cpu`] data structure is set as the `GS` base on the CPU.
//! It currently consists of the following:
//!
//! - GDT
//! - TSS
//! - IST stack spaces

use core::arch::asm;
use core::mem::MaybeUninit;
use core::ptr;

use x86::msr;

use crate::gdt::{GlobalDescriptorTable, TaskStateSegment};
use crate::interrupt::x86_xapic::XAPIC;

static mut NEW_CPU: Cpu = Cpu::new();

/// Size of an IST stack.
const IST_STACK_SIZE: usize = 1 * 1024 * 1024; // 1 MiB

#[repr(C, align(4096))]
pub struct Cpu {

    /// The CPU ID.
    ///
    /// Currently it's the logical APIC ID.
    pub id: usize,

    /// State for the xAPIC driver.
    pub xapic: MaybeUninit<XAPIC>,

    /// The Global Descriptor Table.
    pub gdt: GlobalDescriptorTable,

    /// The Task State Segment.
    pub tss: TaskStateSegment,

    /// The Interrupt Stacks.
    pub ist: [IstStack; 7],


}

/// A stack.
#[repr(transparent)]
pub struct Stack<const SZ: usize>([u8; SZ]);

/// An IST stack.
pub type IstStack = Stack<IST_STACK_SIZE>;


impl<const SZ: usize> Stack<SZ> {
    pub const fn new() -> Self {
        Self([0u8; SZ])
    }

    pub fn bottom(&self) -> *const u8 {
        unsafe {
            (self.0.as_ptr() as *const u8).add(SZ)
        }
    }
}

unsafe impl Send for Cpu {}
unsafe impl Sync for Cpu {}

impl Cpu {
    pub const fn new() -> Self {
        Self {
            // Implement this
            id: 0,
            xapic: MaybeUninit::uninit(),
            gdt: GlobalDescriptorTable::new(),
            tss: TaskStateSegment::new(),
            ist: [
                IstStack::new(),
                IstStack::new(),
                IstStack::new(),
                IstStack::new(),
                IstStack::new(),
                IstStack::new(),
                IstStack::new(),
            ],
        }
    }
}

/// Returns a handle to the current CPU's data structure.
/// We plan to implement support for per-CPU data structures via thread local 
/// variables for now just make sure you have one global CPU data structure and 
/// return it from this method
pub fn get_current() -> &'static mut Cpu {
    // Implement this
    unsafe { &mut *core::ptr::addr_of_mut!(NEW_CPU) }
}

pub fn get_cpu_id() -> i32{
    // Implement this
    unsafe { NEW_CPU.id as i32 }
}


// pub fn get_cpu_id() -> i32{
//     // Implement this
// }
