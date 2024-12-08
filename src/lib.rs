use core::cell::UnsafeCell;
use core::fmt::{Debug, Display};
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashMap;
use std::thread::{Thread, ThreadId};

type Condition<T> = Box<dyn Fn(&T) -> bool + Send>;

type LockSlot<'a, T> = (
    AtomicBool,
    UnsafeCell<Option<parking_lot::MutexGuard<'a, T>>>,
);

type CondMap<T> = HashMap<ThreadId, (Thread, Condition<T>, usize)>;

// TODO: can we get this down to one lock?
pub struct Mutex<T>(parking_lot::Mutex<T>, parking_lot::Mutex<CondMap<T>>);

pub struct MutexGuard<'a, 'b, T>(Option<parking_lot::MutexGuard<'b, T>>, &'a Mutex<T>);

impl<T> Mutex<T> {
    pub fn new(t: T) -> Self {
        Self(t.into(), Default::default())
    }

    pub fn lock(&self) -> MutexGuard<T> {
        let guard = self.0.lock();
        MutexGuard(Some(guard), &self)
    }

    pub fn lock_when<F: Fn(&T) -> bool + Send>(&self, condition: F) -> MutexGuard<T> {
        let guard = self.0.lock();
        if condition(guard.deref()) {
            return MutexGuard(Some(guard), &self);
        }
        drop(guard);

        let thread = std::thread::current();
        let id = thread.id();
        fn boxed<'a, T>(f: impl Fn(&T) -> bool + Send + 'a) -> Box<dyn Fn(&T) -> bool + Send> {
            let f: Box<dyn Fn(&T) -> bool + Send + 'a> = Box::new(f);
            unsafe { core::mem::transmute(f) }
        }
        let condition: Condition<T> = boxed(condition);
        let slot: LockSlot<T> = Default::default();
        let addr = &slot as *const _ as usize;
        let mut mapguard = self.1.lock();
        mapguard.insert(id, (thread, condition, addr));
        drop(mapguard);

        loop {
            std::thread::park();
            if slot.0.load(Ordering::Acquire) {
                let guard = slot.1.into_inner().unwrap();
                return MutexGuard(Some(guard), &self);
            }
        }
    }

    pub fn try_lock(&self) -> Option<MutexGuard<T>> {
        self.0
            .try_lock()
            .map(|guard| MutexGuard(Some(guard), &self))
    }

    fn release(&self, guard: parking_lot::MutexGuard<T>) {
        let mut mapguard = self.1.lock();
        // TODO:  Would extract_if() be faster?  Does it visit the remaining entries when dropped?
        let mut rm = None;
        for (k, v) in mapguard.iter() {
            if v.1(guard.deref()) {
                let slot: &LockSlot<T> = unsafe { &*(v.2 as *const LockSlot<T>) };
                unsafe { *slot.1.get() = Some(guard) };
                slot.0.store(true, Ordering::Release);
                v.0.unpark();
                rm = Some(k.clone());
                break;
            }
        }
        if let Some(ref k) = rm {
            mapguard.remove(k);
        }
    }
}

impl<T: Default> Default for Mutex<T> {
    fn default() -> Mutex<T> {
        Mutex::new(Default::default())
    }
}

impl<T> From<T> for Mutex<T> {
    fn from(t: T) -> Self {
        Mutex::new(t)
    }
}

impl<T: Debug> Debug for Mutex<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Mutex(")?;
        match self.try_lock() {
            Some(guard) => Debug::fmt(guard.deref(), f)?,
            None => f.write_str("<locked>")?,
        }
        f.write_str(")")
    }
}

impl<T> Deref for MutexGuard<'_, '_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.0.as_ref().unwrap().deref()
    }
}

impl<T> DerefMut for MutexGuard<'_, '_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.0.as_mut().unwrap().deref_mut()
    }
}

impl<T> Drop for MutexGuard<'_, '_, T> {
    fn drop(&mut self) {
        self.1.release(self.0.take().unwrap())
    }
}

impl<T: Debug> Debug for MutexGuard<'_, '_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self.deref(), f)
    }
}

impl<T: Display> Display for MutexGuard<'_, '_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.deref(), f)
    }
}

mod tests;
