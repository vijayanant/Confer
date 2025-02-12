# future created by async block is not Send

Let's break down the "future created by async block is not Send" error in Rust,
especially in the context of async/await, Arc<Mutex<>>, and threads. This is a
crucial concept to understand for asynchronous programming in Rust.

## The Problem: Non-Send Types in Futures

The core issue is that futures created by async blocks can capture references
to types that are not thread-safe.  Specifically, they can capture references
to types that do not implement the Send trait.  The Send trait in Rust is a
marker trait that indicates that a type is safe to be moved between threads.

### Why this is a problem in the context of asynchronous code and threads

**async blocks create state machines**: When you use async blocks, the Rust
compiler transforms your code into a state machine. This state machine
represents the execution of your asynchronous operation. It can be paused and
resumed as needed.

**Futures can capture references**:  Futures created by async blocks can
capture references to variables in their surrounding scope. This is how they
access data and perform operations.

**Moving futures between threads**: When you use functions like tokio::spawn or
tokio::task::spawn_blocking, you're essentially moving a future to another
thread for execution.

**The Send trait requirement**: If the future captures a reference to a type
that is not `Send`, then moving that future to another thread becomes unsafe.
The other thread might try to access the captured type while it's being
modified by the original thread, leading to data races and undefined behavior.

### Why `std::sync::Mutex` is not `Send` (and its guard `MutexGuard` even less
so):

The standard `std::sync::Mutex` and its associated `MutexGuard` are not `Send`.
This is because they are designed for synchronous, blocking operations. The
`MutexGuard` is specifically tied to the thread that acquired the lock. Moving
it to another thread would violate the fundamental principle of mutexes (mutual
exclusion).

### Why `tokio::sync::Mutex` is `Send`:

The `tokio::sync::Mutex` is specifically designed for asynchronous operations
and is `Send` and `Sync`. Its `MutexGuard` is `Send`. This is because it uses
asynchronous primitives internally and is aware of the Tokio runtime.  It's
safe to move the future that holds the `tokio::sync::Mutex` guard between
threads within the Tokio runtime.

### Why `Arc<Mutex<>>` alone is not enough:

While `Arc` itself is `Send` and `Sync` (meaning it can be safely shared across
threads), the guard returned by `lock()` on a `std::sync::Mutex` is not `Send`.
So, even if you clone the `Arc` and move it into an async block, the act of
acquiring the lock (which gives you a `MutexGuard`) creates a problem because
the guard is not `Send`.

## The Solutions:

### `tokio::sync::Mutex` (The Best Solution):

Use tokio::sync::Mutex instead of std::sync::Mutex. This mutex is designed for
asynchronous operations and is Send and Sync. This is the most efficient and
idiomatic solution for asynchronous code.

### `tokio::task::spawn_blocking` (Workaround):

Use `tokio::task::spawn_blocking` to run the blocking operation (acquiring the
`std::sync::Mutex` lock) on a separate thread pool. This works, but it's less
efficient than using `tokio::sync::Mutex` directly. It also requires you to
make your `HashMapDataStore` methods non-async.


