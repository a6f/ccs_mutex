use core::fmt::{Debug, Display};
use core::ops::{Deref, DerefMut};
use core::time::Duration;

pub struct Mutex<T>(parking_lot::Mutex<T>, parking_lot::Condvar);

pub struct MutexGuard<'a, 'b, T>(parking_lot::MutexGuard<'b, T>, &'a parking_lot::Condvar);

impl<T> Mutex<T> {
    pub fn new(t: T) -> Self {
        Self(t.into(), Default::default())
    }

    pub fn lock(&self) -> MutexGuard<T> {
        let guard = self.0.lock();
        MutexGuard(guard, &self.1)
    }

    pub fn lock_when(&self, condition: impl Fn(&T) -> bool) -> MutexGuard<T> {
        let mut guard = self.0.lock();
        self.1.wait_while(&mut guard, |t| !condition(t));
        MutexGuard(guard, &self.1)
    }

    pub fn lock_when_with_timeout(
        &self,
        condition: impl Fn(&T) -> bool,
        timeout: Duration,
    ) -> Option<MutexGuard<T>> {
        let mut guard = self.0.lock();
        let timed_out = self
            .1
            .wait_while_for(&mut guard, |t| !condition(t), timeout);
        match timed_out.timed_out() {
            true => None,
            false => Some(MutexGuard(guard, &self.1)),
        }
    }

    pub fn try_lock(&self) -> Option<MutexGuard<T>> {
        self.0.try_lock().map(|guard| MutexGuard(guard, &self.1))
    }
}

impl<T> MutexGuard<'_, '_, T> {
    pub fn await_condition(&mut self, condition: impl Fn(&T) -> bool) {
        self.1.wait_while(&mut self.0, |t| !condition(t));
    }

    pub fn await_with_timeout(
        &mut self,
        condition: impl Fn(&T) -> bool,
        timeout: Duration,
    ) -> bool {
        !self
            .1
            .wait_while_for(&mut self.0, |t| !condition(t), timeout)
            .timed_out()
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
        self.0.deref()
    }
}

impl<T> DerefMut for MutexGuard<'_, '_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.0.deref_mut()
    }
}

impl<T> Drop for MutexGuard<'_, '_, T> {
    fn drop(&mut self) {
        self.1.notify_all();
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
