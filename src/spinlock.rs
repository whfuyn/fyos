use core::cell::UnsafeCell;
use core::marker::Sync;
use core::ops::Deref;
use core::ops::DerefMut;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;

pub struct SpinLock<T: ?Sized> {
    is_locked: AtomicBool,
    value: UnsafeCell<T>,
}

pub struct SpinLockGuard<'a, T: ?Sized>(&'a SpinLock<T>);

impl<T> SpinLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            is_locked: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }
}

impl<T: ?Sized> SpinLock<T> {
    pub fn lock(&self) -> SpinLockGuard<T> {
        // TODO: Not quite sure about the Ordering, check these later.
        while self
            .is_locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Acquire)
            .is_err()
        {
            core::hint::spin_loop();
        }

        SpinLockGuard(self)
    }
}

impl<'a, T: ?Sized> Drop for SpinLockGuard<'a, T> {
    fn drop(&mut self) {
        assert!(self
            .0
            .is_locked
            .compare_exchange(true, false, Ordering::Acquire, Ordering::Acquire)
            .is_ok())
    }
}

impl<'a, T: ?Sized> Deref for SpinLockGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // Safety: This is guarded by the atomic flag `locked` in the SpinLock.
        unsafe { &*self.0.value.get() as &Self::Target }
    }
}

impl<'a, T: ?Sized> DerefMut for SpinLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Safety: This is guarded by the atomic flag `locked` in the SpinLock.
        unsafe { &mut *self.0.value.get() as &mut Self::Target }
    }
}

// Safety:
// Thoes conditions are copied from std Mutex. I'm not 100% sure why T: Send is
// needed and sufficient.
// But thread_local data might be an example that !Send data should not be Send
// or Sync even in Mutex.
unsafe impl<T: ?Sized + Send> Send for SpinLock<T> {}
unsafe impl<T: ?Sized + Send> Sync for SpinLock<T> {}
