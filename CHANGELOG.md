# Change Log

## [0.3.0]

### New - 2025-01-09

* New `ConcurrentWrapper` structure that makes it possible to
  log from multiple threads.

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
