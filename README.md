## Conditional Critical Sections

A wrapper for `std::sync::Mutex` which adds the Conditional Critical Sections
interface of [nsync](https://github.com/google/nsync) or
[abseil Mutex](https://abseil.io/docs/cpp/guides/synchronization#conditional-critical-sections).
Each `Mutex` embeds a `Condvar`, and the `Condvar` is signaled on each unlock,
so you don't have to keep track of which operations might wake each waiter.

An implementation could be further optimized by evaluating the registered
conditions during unlock, rather than making each waiter wake up to check its
own condition.  This requires that conditions be `Send`, and some cooperation
from `Mutex` to get an overall speedup in typical use.
