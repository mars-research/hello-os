// src/sync.rs (or inside memory/mod.rs if you prefer)
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};
use x86_64::instructions::interrupts;

pub struct IrqGuard {
    was_enabled: bool,
}
impl IrqGuard {
    #[inline]
    pub fn acquire() -> Self {
        let was = interrupts::are_enabled();
        if was { interrupts::disable(); }
        IrqGuard { was_enabled: was }
    }
}
impl Drop for IrqGuard {
    #[inline]
    fn drop(&mut self) {
        if self.was_enabled {
            // Restore to the previous state only if it was on.
            interrupts::enable();
        }
    }
}

pub struct RawSpinLock {
    flag: AtomicBool,
}
impl RawSpinLock {
    pub const fn new() -> Self { Self { flag: AtomicBool::new(false) } }
    #[inline]
    fn lock(&self) {
        while self.flag.swap(true, Ordering::Acquire) {
            core::hint::spin_loop();
        }
    }
    #[inline]
    fn unlock(&self) {
        self.flag.store(false, Ordering::Release);
    }
}

pub struct Mutex<T> {
    lock: RawSpinLock,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}

pub struct MutexGuard<'a, T> {
    m: &'a Mutex<T>,
    _irq: IrqGuard, // keeps IRQs disabled while guard is alive
}

impl<T> Mutex<T> {
    pub const fn new(val: T) -> Self {
        Self { lock: RawSpinLock::new(), data: UnsafeCell::new(val) }
    }
    #[inline]
    pub fn lock(&self) -> MutexGuard<'_, T> {
        let irq = IrqGuard::acquire();   // save/disable IRQs
        self.lock.lock();                // spinlock
        MutexGuard { m: self, _irq: irq }
    }
    #[inline]
    fn unlock(&self) {
        self.lock.unlock();
        // _irq guard drops next and restores previous IRQ state
    }
    #[inline]
    pub fn get_mut(&self) -> &mut T {
        // Only for init, before concurrency
        unsafe { &mut *self.data.get() }
    }
}
impl<'a, T> core::ops::Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target { unsafe { &*self.m.data.get() } }
}
impl<'a, T> core::ops::DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target { unsafe { &mut *self.m.data.get() } }
}
impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) { self.m.unlock(); }
}
