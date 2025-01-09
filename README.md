# DSI Progress Logger

[![downloads](https://img.shields.io/crates/d/dsi-progress-logger)](https://crates.io/crates/dsi-progress-logger)
[![dependents](https://img.shields.io/librariesio/dependents/cargo/dsi-progress-logger)](https://crates.io/crates/dsi-progress-logger/reverse_dependencies)
![GitHub CI](https://github.com/vigna/dsi-progress-logger-rs/actions/workflows/rust.yml/badge.svg)
![license](https://img.shields.io/crates/l/dsi-progress-logger)
[![](https://tokei.rs/b1/github/vigna/dsi-progress-logger-rs?type=Rust,Python)](https://github.com/vigna/dsi-progress-logger-rs)
[![Latest version](https://img.shields.io/crates/v/dsi-progress-logger.svg)](https://crates.io/crates/dsi-progress-logger)
[![Documentation](https://docs.rs/dsi-progress-logger/badge.svg)](https://docs.rs/dsi-progress-logger)

A tunable progress logger to log progress information about long-running
activities.

It is a port of the Java class [`it.unimi.dsi.util.ProgressLogger`] from the
[DSI Utilities]. Logging is based on the standard [`log`] crate at the `info`
level.

There is a [`ProgressLog`] trait and a default implementation
[`ProgressLogger`].

## Concurrent Logging

A [`ProgressLogger`] is not thread-safe. If you need to log from multiple
threads, you can use [`ConcurrentWrapper`] to wrap a [`ProgressLog`]
implementation. [`ConcurrentWrapper`] implements [`ProgressLog`], but it
features also an additional method [`spawn`] that returns a new thread-safe
[`ConcurrentWrapper`] with the same underlying [`ProgressLog`] implementation
that can be passed to other threads. Convenience constructors and macros make
concurrent progress logging as easy as single-threaded logging.

## Optional Logging

This crate supports optional logging by implementing [`ProgressLog`] for
`Option<ProgressLog>::None` as a no-op. As a result, you can pass to functions
an argument `pl` that is a `&mut impl ProgressLog`, with the following behavior:

- if you pass a `&mut ProgressLogger`, the progress logger will be used, without
  any check;
- if you pass a `&mut Option::<ProgressLogger>::None`, no
  logging will be performed, and in fact the logging code should be entirely
  optimized away by the compiler; the macro [`no_logging!`], which expands
  to `&mut Option::<ProgressLogger>::None`, can be used a convenient way to
  switch off logging;
- if you pass an `&mut Option<ProgressLogger>`, logging will happen depending on
  the variant, and there will be a runtime check for each call.

There is an [`info`] method that can be used to log information to the logger at
the `info` level. The advantage of using [`info`] is that the logging will be
optional depending on the type of the logger.

## Cloning

The [`clone`] method will return a logger with the same setup but with all the
counters reset. This is useful when you want to configure a logger and then use
its configuration for other loggers.

## Acknowledgments

This software has been partially supported by project SERICS (PE00000014) under
the NRRP MUR program funded by the EU - NGEU. Views and opinions expressed are
however those of the authors only and do not necessarily reflect those of the
European Union or the Italian MUR. Neither the European Union nor the Italian
MUR can be held responsible for them.

[`ProgressLog`]: https://docs.rs/dsi-progress-logger/latest/dsi_progress_logger/trait.ProgressLog.html
[`ProgressLogger`]: https://docs.rs/dsi-progress-logger/latest/dsi_progress_logger/struct.ProgressLogger.html
[`ConcurrentWrapper`]: <https://docs.rs/dsi-progress-logger/latest/dsi_progress_logger/struct.ConcurrentWrapper.html>
[`spawn`]: <https://docs.rs/dsi-progress-logger/latest/dsi_progress_logger/struct.ConcurrentWrapper.html#tymethod.spawn>
[`info`]: https://docs.rs/dsi-progress-logger/latest/dsi_progress_logger/trait.ProgressLog.html#tymethod.info
[`clone`]: https://docs.rs/dsi-progress-logger/latest/dsi_progress_logger/trait.ProgressLog.html#tymethod.clone
[`it.unimi.dsi.util.ProgressLogger`]: https://dsiutils.di.unimi.it/docs/it/unimi/dsi/logging/ProgressLogger.html
[DSI Utilities]: https://dsiutils.di.unimi.it/
[`log`]: https://docs.rs/log
[`no_logging!`]: <https://docs.rs/dsi-progress-logger/latest/dsi_progress_logger/macro.no_logging.html>
