use core::cell::UnsafeCell;
use core::marker::Sync;
use core::mem::MaybeUninit;
use core::ops::Deref;
use core::sync::atomic::AtomicU8;
use core::sync::atomic::Ordering;

struct InitStage;

impl InitStage {
    #![allow(non_upper_case_globals)]

    const Uninit: u8 = 0;
    const Initing: u8 = 1;
    const Inited: u8 = 2;
}

pub struct LazyStatic<T: 'static, F: FnOnce() -> T> {
    init_state: AtomicU8,
    init_fn: UnsafeCell<Option<F>>,

    value: UnsafeCell<MaybeUninit<T>>,
}

impl<T: 'static, F: FnOnce() -> T> LazyStatic<T, F> {
    pub const fn new(init_fn: F) -> Self {
        Self {
            init_state: AtomicU8::new(InitStage::Uninit),
            init_fn: UnsafeCell::new(Some(init_fn)),
            value: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }
}

impl<T: 'static, F: FnOnce() -> T> Drop for LazyStatic<T, F> {
    fn drop(&mut self) {
        match self.init_state.load(Ordering::Relaxed) {
            InitStage::Uninit | InitStage::Initing => (),
            // SAFETY:
            // - We have unique access to self.value on drop, and
            // that value has been inited.
            InitStage::Inited => unsafe {
                (*self.value.get()).assume_init_drop();
            },
            _ => unreachable!(),
        }
    }
}

impl<T: 'static, F: FnOnce() -> T> Deref for LazyStatic<T, F> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // TODO:
        // This may cause a race condition when the interrupt handler gets in
        // during the Initing stage.
        // We need to block that temporarily.
        loop {
            match self.init_state.compare_exchange(
                InitStage::Uninit,
                InitStage::Initing,
                Ordering::Acquire,
                Ordering::Acquire,
            ) {
                // SAFETY:
                // - We have unique access to both self.value and self.init_fn.
                // - This is the only time that self.init_fn's ownership got taken.
                Ok(_) => unsafe {
                    let init_fn = (*self.init_fn.get()).take().unwrap();
                    (*self.value.get()).write((init_fn)());
                    self.init_state.store(InitStage::Inited, Ordering::Release);
                    return (*self.value.get()).assume_init_ref();
                },
                Err(InitStage::Initing) => {
                    core::hint::spin_loop();
                }
                // SAFETY:
                // - There won't be any ohter mutable refs to self.value, and
                // - The value has been initialized.
                Err(InitStage::Inited) => unsafe {
                    return (*self.value.get()).assume_init_ref();
                },
                _ => unreachable!(),
            }
        }
    }
}

// TODO: check SAFETY
// SAFETY: I'm not sure...
unsafe impl<T: Send + 'static, F: Send + FnOnce() -> T> Send for LazyStatic<T, F> {}
unsafe impl<T: Send + Sync + 'static, F: Send + Sync + FnOnce() -> T> Sync for LazyStatic<T, F> {}

#[macro_export]
macro_rules! lazy_static {
    ($vis:vis static ref $name:ident: $ty:ty = $expr:expr;) => {
        $vis static $name: $crate::lazy_static::LazyStatic::<$ty, fn() -> $ty> =
            $crate::lazy_static::LazyStatic::<$ty, fn() -> $ty>::new(|| $expr);
    };
    ($vis:vis static ref $name:ident: $ty:ty = $expr:expr; $($rest:tt)*) => {
        $crate::lazy_static!{
            $vis static ref $name: $ty = $expr;
        }
        $crate::lazy_static!{ $($rest)* }
    };
}
