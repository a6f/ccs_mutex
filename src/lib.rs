use core::fmt::{Debug, Display};
use core::ops::{Deref, DerefMut};
use core::time::Duration;
pub use std::sync::WaitTimeoutResult;

pub struct Mutex<T>(std::sync::Mutex<T>, std::sync::Condvar);

pub struct MutexGuard<'a, 'b, T>(Option<std::sync::MutexGuard<'b, T>>, &'a std::sync::Condvar);

impl<T> Mutex<T> {
    pub fn new(t: T) -> Self {
        Self(t.into(), Default::default())
    }

    pub fn lock(&self) -> MutexGuard<T> {
        let guard = self.0.lock().unwrap();
        MutexGuard(Some(guard), &self.1)
    }

    pub fn lock_when(&self, condition: impl Fn(&T) -> bool) -> MutexGuard<T> {
        let guard = self.0.lock().unwrap();
        let guard = self.1.wait_while(guard, |t| !condition(t)).unwrap();
        MutexGuard(Some(guard), &self.1)
    }

    pub fn lock_when_with_timeout(
        &self,
        condition: impl Fn(&T) -> bool,
        timeout: Duration,
    ) -> Result<MutexGuard<T>, WaitTimeoutResult> {
        let guard = self.0.lock().unwrap();
        let (guard, timed_out) = self
            .1
            .wait_timeout_while(guard, timeout, |t| !condition(t))
            .unwrap();
        match timed_out.timed_out() {
            true => Err(timed_out),
            false => Ok(MutexGuard(Some(guard), &self.1)),
        }
    }

    pub fn try_lock(&self) -> Option<MutexGuard<T>> {
        self.0
            .try_lock()
            .ok()
            .map(|guard| MutexGuard(Some(guard), &self.1))
    }
}

impl<T> MutexGuard<'_, '_, T> {
    pub fn await_condition(&mut self, condition: impl Fn(&T) -> bool) {
        let guard = self.0.take().unwrap();
        let guard = self.1.wait_while(guard, |t| !condition(t)).unwrap();
        self.0 = Some(guard);
    }

    pub fn await_with_timeout(
        &mut self,
        condition: impl Fn(&T) -> bool,
        timeout: Duration,
    ) -> bool {
        let guard = self.0.take().unwrap();
        let (guard, timed_out) = self
            .1
            .wait_timeout_while(guard, timeout, |t| !condition(t))
            .unwrap();
        self.0 = Some(guard);
        !timed_out.timed_out()
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
