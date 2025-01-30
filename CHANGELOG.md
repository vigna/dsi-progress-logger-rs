# Change Log

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
