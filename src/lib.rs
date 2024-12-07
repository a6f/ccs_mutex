use core::fmt::{Debug, Display};
use core::ops::{Deref, DerefMut};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::thread::{Thread, ThreadId};

type Condition<T> = Box<dyn Fn(&T) -> bool + Send>;

type CondMap<T> = HashMap<ThreadId, (Thread, Condition<T>)>;

pub struct Mutex<T>(std::sync::Mutex<T>, std::sync::Mutex<CondMap<T>>);

pub struct MutexGuard<'a, 'b, T>(Option<std::sync::MutexGuard<'b, T>>, &'a Mutex<T>);

impl<T> Mutex<T> {
    pub fn new(t: T) -> Self {
        Self(t.into(), Default::default())
    }

    pub fn lock(&self) -> MutexGuard<T> {
        let guard = self.0.lock().unwrap();
        MutexGuard(Some(guard), &self)
    }

    pub fn lock_when<F: Fn(&T) -> bool + Send>(&self, condition: F) -> MutexGuard<T> {
        let guard = self.0.lock().unwrap();
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
        let mut mapguard = self.1.lock().unwrap();
        mapguard.insert(id, (thread, condition));
        drop(mapguard);

        loop {
            std::thread::park();
            let guard = self.0.lock().unwrap();
            let mut mapguard = self.1.lock().unwrap();
            let Entry::Occupied(entry) = mapguard.entry(id) else {
                panic!();
            };
            // TODO:  It's a pity we have to retake the lock and recheck the condition.
            // parking_lot::MutexGuard can be Send; try implementing in terms of that.
            if entry.get().1(guard.deref()) {
                let _dealloc = entry.remove();
                drop(mapguard);
                return MutexGuard(Some(guard), &self);
            }
        }
    }

    pub fn try_lock(&self) -> Option<MutexGuard<T>> {
        self.0
            .try_lock()
            .ok()
            .map(|guard| MutexGuard(Some(guard), &self))
    }

    fn release(&self, guard: std::sync::MutexGuard<T>) {
        let mapguard = self.1.lock().unwrap();
        for v in mapguard.values() {
            if v.1(guard.deref()) {
                v.0.unpark();
                break;
            }
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
