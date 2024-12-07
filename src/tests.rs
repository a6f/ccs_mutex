#![cfg(test)]

use super::*;

#[derive(Default)]
struct PCQ<T>(Mutex<std::collections::VecDeque<T>>);

impl<T> PCQ<T> {
    fn push(&self, t: T) {
        self.0.lock().push_back(t);
    }

    fn push_n<const N: usize>(&self, a: [T; N]) {
        let mut guard = self.0.lock();
        for t in a {
            guard.push_back(t);
        }
    }

    fn pop(&self) -> T {
        self.0.lock_when(|q| !q.is_empty()).pop_front().unwrap()
    }

    fn pop_n<const N: usize>(&self) -> [T; N] {
        let mut guard = self.0.lock_when(|q| q.len() >= N);
        [(); N].map(|_| guard.pop_front().unwrap())
    }
}

#[test]
fn pcq_run_benchmarks() {
    measure(|| pcq_benchmark::<1, 1>());
    measure(|| pcq_benchmark::<1, 10>());
    measure(|| pcq_benchmark::<10, 1>());
    measure(|| pcq_benchmark::<10, 10>());
}

fn measure(x: impl FnOnce()) {
    let t0 = std::time::Instant::now();
    x();
    println!("{:?}", t0.elapsed());
}

fn pcq_benchmark<const PUSH_N: usize, const POP_N: usize>() {
    print!("testing push {PUSH_N} pop {POP_N}... ");
    let q = PCQ::<usize>::default();
    std::thread::scope(|scope| {
        const TOTAL: usize = 10_000_000;
        scope.spawn(|| {
            for i in (0..TOTAL).step_by(PUSH_N) {
                if PUSH_N == 1 {
                    q.push(i);
                } else {
                    let arr: [usize; PUSH_N] = core::array::from_fn(|j| i + j);
                    q.push_n(arr);
                }
            }
        });
        for _ in (0..TOTAL).step_by(POP_N) {
            if POP_N == 1 {
                q.pop();
            } else {
                q.pop_n::<POP_N>();
            }
        }
    });
}
