use core::ops::{Deref, DerefMut};
use std::sync::{Condvar, Mutex, MutexGuard, WaitTimeoutResult};
use std::time::Duration;

pub struct CondMutex<T>(Mutex<T>, Condvar);

pub struct CondGuard<'a, 'b, T>(Option<MutexGuard<'b, T>>, &'a Condvar);

impl<T> CondMutex<T> {
    pub fn new(t: T) -> Self {
        Self(Mutex::new(t), Condvar::new())
    }

    pub fn lock(&self) -> CondGuard<T> {
        let guard = self.0.lock().unwrap();
        CondGuard(Some(guard), &self.1)
    }

    pub fn lock_when(&self, condition: impl Fn(&T) -> bool) -> CondGuard<T> {
        let guard = self.0.lock().unwrap();
        let guard = self.1.wait_while(guard, |t| condition(t)).unwrap();
        CondGuard(Some(guard), &self.1)
    }

    pub fn lock_when_with_timeout(
        &self,
        condition: impl Fn(&T) -> bool,
        timeout: Duration,
    ) -> Result<CondGuard<T>, WaitTimeoutResult> {
        let guard = self.0.lock().unwrap();
        let (guard, timed_out) = self
            .1
            .wait_timeout_while(guard, timeout, |t| condition(t))
            .unwrap();
        match timed_out.timed_out() {
            true => Err(timed_out),
            false => Ok(CondGuard(Some(guard), &self.1)),
        }
    }

    pub fn try_lock(&self) -> Option<CondGuard<T>> {
        self.0
            .try_lock()
            .ok()
            .map(|guard| CondGuard(Some(guard), &self.1))
    }
}

impl<T> CondGuard<'_, '_, T> {
    pub fn await_condition(&mut self, condition: impl Fn(&T) -> bool) {
        let guard = self.0.take().unwrap();
        let guard = self.1.wait_while(guard, |t| condition(t)).unwrap();
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
            .wait_timeout_while(guard, timeout, |t| condition(t))
            .unwrap();
        self.0 = Some(guard);
        !timed_out.timed_out()
    }
}

impl<T> Deref for CondGuard<'_, '_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.0.as_ref().unwrap().deref()
    }
}

impl<T> DerefMut for CondGuard<'_, '_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.0.as_mut().unwrap().deref_mut()
    }
}

impl<T> Drop for CondGuard<'_, '_, T> {
    fn drop(&mut self) {
        self.1.notify_all()
    }
}
