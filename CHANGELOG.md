# Change Log

## unreleased

### New

- New `ProgressLog::format` setter and `Format` enum to choose how counts,
  speeds, and per-item timings are displayed: plain, thousands separated, or
  thousands separated with an SI prefix for speeds and timings (e.g.,
  `346.59M nodes/s`). In particular, counts can now be thousands separated
  even if a fixed time unit has been set with `ProgressLog::time_unit`.

### Improved

- By default, speeds and per-item timings are now thousands separated (e.g.,
  `346,588,344.72 nodes/s`), like counts, unless a fixed time unit has been
  set with `ProgressLog::time_unit`.

### Fixed

- `scale` and `humanize` now keep values of 10²⁷ or more in yottas:
  previously, they divided them once more by 1000 after reaching the `Y`
  prefix, so they displayed them 1000 times too small.

## [0.8.9] - 2026-07-23

### Changed

- License is now MIT or Apache.

## [0.8.8] - 2026-07-16

### New

- New `ConcurrentWrapper::MAX_THRESHOLD` constant: thresholds are now
  clamped to it, which guarantees that the internal counter used by
  `light_update` cannot overflow.

- `Display` is now implemented for `ConcurrentWrapper<P>` for all
  `P: ProgressLog + Display`, not just for the default type parameter.

### Fixed

- `ProgressLog::refresh` now refreshes the available/free memory
  statistics, too: previously, they were read only when memory display
  was enabled, so they would become stale during long activities.

- `ConcurrentWrapper::flush` is now a no-op when the local count is
  zero. In particular, dropping a wrapper after `stop`/`done` no longer
  emits a spurious log line.

- Dropping a `ConcurrentWrapper` no longer panics if the mutex guarding
  the underlying logger has been poisoned by a panic in another thread.

- The `Option<P>` and `&mut P` implementations of `ProgressLog` now
  forward `update_with_count` directly: previously, the default method
  would read the clock even when no logging was performed.

- `ConcurrentWrapper::update_with_count_and_time` now uses the provided
  time instead of reading the clock again when flushing.

## [0.8.7] - 2026-04-29

### New

- New `push_log_target`/`pop_log_target` methods that make it easier
  to log subtasks.

### Fixed

- `ConcurrentProgressLogger` was not flushing the internal counter
  before `stop`/`done`.

## [0.8.6] - 2026-04-03

### New

- New `ProgressLog::log_level` setter to configure the log level
  used for progress messages (default: `Info`).

- `ProgressLog::expected_updates` now takes an
  `impl Into<Option<usize>>` instead of an `Option<usize>`, so it
  can be called with a `usize` directly. The same applies to
  `ProgressLog::time_unit`.

### Fixed

- `ProgressLogger::clone` now preserves `log_target`.

### Changed

- Pinned `sysinfo` to 0.36.1, restoring Rust 1.85 compatibility.

## [0.8.4] - 2026-02-15 [YANKED]

### Changed

- Bumped Rust version to 1.88 as required by `sysinfo`.

## [0.8.3] - 2026-02-15 [YANKED]

### Fixed

- `ProgressLog::elapsed` now returns the elapsed time after
  a stop, instead of the time since the last start.

### Changed

- Updated to `sysinfo` 0.38.

## [0.8.2] - 2025-07-01

### New

- `ConcurrentWrapper::flush` now forces a log of the underlying logger, so
  even if parallelism brings the number of items per clone below the
  logging threshold the user will see some output.

## [0.8.1] - 2025-03-08

### New

- The plural of an item name is now cached as its computation is expensive.

- Removed `multithread` feature from `sysinfo` dependency, and
  documented deadlock problems with `rayon` and `sysinfo`
  when using `display_memory`.

## [0.8.0] - 2025-02-20

### New

- Added missing `ProgressLog::count` method.

## [0.7.0] - 2025-02-16

### New

- New methods `ProgressLog::trace`, `ProgressLog::debug`, `ProgressLog::warn`,
  and `ProgressLog::error` with the same logic of `ProgressLog::info`, but for
  different log levels.

## [0.6.0] - 2025-02-04

### New

- New `ConcurrentLog::update_with_count_and_time` internal method that makes it
  possible to move the call to `Instant::now` out of the critical section in
  `ConcurrentWrapper`.

## [0.5.1] - 2025-02-03

### Fixed

- `ConcurrentWrapper::update_light` was adding the local count twice.

## [0.5.0] - 2025-01-30

### New

- New `ProgressLog::concurrent` method to derive a `ConcurrentProgressLog` from
  a `ProgressLog`.

## [0.4.0] - 2025-01-28

### New

- New `ConcurrentProgressLog` trait analogous to `ProgressLog`.

- New `ProgressLog::add_to_count` method.

### Improved

- The `no_logging!` macro now works with `ConcurrentProgressLog`.

- Updated to latest `sysinfo` crate.

## [0.3.0] - 2025-01-10

### New

- New `ConcurrentWrapper` structure that makes it possible to
  log from multiple threads.

- `ProgressLog` is now implemeneted for `&mut P`, given that
  `P: ProgressLog`.

## [0.2.5] - 2024-11-06

### New

- `no_logging!` macro.

## [0.2.4] - 2024-03-24

### New

- Now the logging target is configurable. Thanks to Valentin
  Lorentz for implementing this feature.

- A `progress_logger!` macro makes initialization easier, and defaults
  the logging target to `std::module_path!()`.

## [0.2.3] - 2024-03-18

### New

- Added prelude.
