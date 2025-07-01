# Change Log

## [0.8.2] - 2025-07-01

### New

* `ConcurrentWrapper::flush` now forces a log of the underlying logger, so
  even if parallelism brings the number of items per clone below the
  logging threshold the user will see some output.

## [0.8.1] - 2025-03-08

### New

* The plural of an item name is now cached as its computation is expensive.

* Removed `multithread` feature from `sysinfo` dependency, and
  documented deadlock problems with `rayon` and `sysinfo`
  when using `display_memory`.

## [0.8.0] - 2025-02-20

### New

* Added missing `ProgressLog::count` method.

## [0.7.0] - 2025-02-16

### New

* New methods `ProgressLog::trace`, `ProgressLog::debug`,  `ProgressLog::warn`,
  and `ProgressLog::error` with the same logic of  `ProgressLog::info`, but for
  different log levels.

## [0.6.0] - 2025-02-04

### New

* New `ConcurrentLog::update_with_count_and_time` internal method that makes it
  possible to move the call to `Instant::now` out of the critical section in
  `ConcurrentWrapper`.

## [0.5.1] - 2025-02-03

### Fixed

* `ConcurrentWrapper::update_light` was adding the local count twice.

## [0.5.0] - 2025-01-30

### New

* New `ProgressLog::concurrent` method to derive a `ConcurrentProgressLog` from
  a `ProgressLog`.

## [0.4.0] - 2025-01-28

### New

* New `ConcurrentProgressLog` trait analogous to `ProgressLog`.

* New `ProgressLog::add_to_count` method.

### Improved

* The `no_logging!` macro now works with `ConcurrentProgressLog`.

* Updated to latest `sysinfo` crate.

## [0.3.0] - 2025-01-10

### New

* New `ConcurrentWrapper` structure that makes it possible to
  log from multiple threads.

* `ProgressLog` is now implemeneted for `&mut P`, given that
  `P: ProgressLog`.

## [0.2.5] - 2024-11-06

### New

* `no_logging!` macro.

## [0.2.4] - 2024-03-24

### New

* Now the logging target is configurable. Thanks to Valentin
  Lorentz for implementing this feature.

* A `progress_logger!` macro makes initialization easier, and defaults
  the logging target to `std::module_path!()`.

## [0.2.3] - 2024-03-18

### New

* Added prelude.
