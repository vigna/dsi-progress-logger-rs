# DSI Progress Logger

[![downloads](https://img.shields.io/crates/d/dsi-progress-logger)](https://crates.io/crates/dsi-progress-logger)
[![dependents](https://img.shields.io/librariesio/dependents/cargo/dsi-progress-logger)](https://crates.io/crates/dsi-progress-logger/reverse_dependencies)
![GitHub CI](https://github.com/vigna/dsi-progress-logger-rs/actions/workflows/rust.yml/badge.svg)
![license](https://img.shields.io/crates/l/dsi-progress-logger)
[![](https://tokei.rs/b1/github/vigna/dsi-progress-logger-rs?type=Rust,Python)](https://github.com/vigna/dsi-progress-logger-rs)
[![Latest version](https://img.shields.io/crates/v/dsi-progress-logger.svg)](https://crates.io/crates/dsi-progress-logger)
[![Documentation](https://docs.rs/dsi-progress-logger/badge.svg)](https://docs.rs/dsi-progress-logger)

A tunable progress logger to log progress information about long-running activities.

It is a port of the Java class [`it.unimi.dsi.util.ProgressLogger`] from the
[DSI Utilities]. Logging is based on the standard [`log`] crate at the `info`
level.

There is a [`ProgressLog`] trait and a default implementation
[`ProgressLogger`].

To log the progress of an activity, you call [`start`]. Then, each time you want
to mark progress, you call [`update`], which increases the item counter, and
will log progress information if enough time has passed since the last log.
[`light_update`] will perform a time check only on updates multiples of
[`LIGHT_UPDATE_MASK`] + 1; it  should be used when the activity has an extremely
low cost that is comparable to that of the time check (a call to
[`Instant::now()`] itself.

A few setters can be called at any time to customize the logger (e.g.,
[`item_name`], [`log_interval`], [`expected_updates`], etc.). The setters take
and return a mutable reference to the logger, so you must first assign the
logger to a variable, and then you can chain-call the setters on the variable in
fluent style. The disadvantage of this approach is that you must assign the
logger to a variable, but the advantage is that you can call any setter without
having to reassign the variable holding the logger. There is also a
[`progress_logger!`] macro described later.

It is also possible to log used and free memory at each log interval by calling
[`display_memory`]. Memory is read from system data by the [`sysinfo`] crate,
and will be updated at each log interval (note that this will slightly slow down
the logging process).

At any time, displaying the progress logger will give you time information up to
the present. However,  since it is impossible to update the memory information
from the [`Display::fmt`] implementation, you should call [`refresh`] before
displaying the logger on your own.

When the activity is over, you call [`stop`], which fixes the final time, and
possibly display again the logger. [`done`] will stop the logger, print
`Completed.`, and display the final stats.

After you finish a run of the progress logger, can call [`start`] again to
measure another activity.

A typical call sequence to a progress logger is as follows:

```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use dsi_progress_logger::prelude::*;

stderrlog::new().verbosity(2).init()?;

let mut pl = ProgressLogger::default();
pl.item_name("pumpkin");
pl.start("Smashing pumpkins...");
for _ in 0..100 {
   // do something on each pumpkin
   pl.update();
}
pl.done();
#     Ok(())
# }
```

The [`progress_logger`] macro will create the progress logger for you and set
its [`log_target`] to [`std::module_path!()`], which is usually what you want.
You may also call any setter with a key-value syntax:

```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use dsi_progress_logger::prelude::*;

stderrlog::new().verbosity(2).init()?;

let mut pl = progress_logger!(item_name="pumpkin");
pl.start("Smashing pumpkins...");
for _ in 0..100 {
   // do something on each pumpkin
   pl.update();
}
pl.done();
#     Ok(())
# }
```

A progress logger can also be used as a handy timer:

```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use dsi_progress_logger::prelude::*;

stderrlog::new().verbosity(2).init()?;

let mut pl = progress_logger!(item_name="pumpkin");
pl.start("Smashing pumpkins...");
for _ in 0..100 {
   // do something on each pumpkin
}
pl.done_with_count(100);
#     Ok(())
# }
```

This progress logger will display information about  memory usage:

```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use dsi_progress_logger::prelude::*;

stderrlog::new().verbosity(2).init()?;

let mut pl = progress_logger!(display_memory=true);
#     Ok(())
# }
```

## Optional logging

This crate supports optional logging by implementing [`ProgressLog`] for `Option<ProgressLog>` as a no-op.
As a result, you can pass to functions an argument `pl` that is an `impl ProgressLog`, with the following behavior:

- if you pass a [`ProgressLogger`], the progress logger will be used, without any check;
- if you pass `Option::<ProgressLogger>::None`, no logging will be performed, and in fact the logging
  code should be entirely optimized away by the compiler;
- if you pass an `Option<ProgressLogger>`, logging will happen depending on the variant, and there
  will be a runtime check for each call to `pl`.

There is an [`info`] method that can be used to log information to the logger
at the `info` level.
The advantage of using [`info`] is that the
logging will be optional depending on the type of the logger.

## Cloning

The [`clone`] method will return a logger with the same setup but with all the counters reset.
This is useful when you want to configure a logger and then use its configuration for other loggers.

Note that this method is part of [`ProgressLog`]: otherwise, because of the orphan rule
we would not be able to implement it for `Option<ProgressLog>`.

## Acknowledgments

This software has been partially supported by project SERICS (PE00000014) under
the NRRP MUR program funded by the EU - NGEU.

[`ProgressLog`]: https://docs.rs/dsi-progress-logger/latest/dsi_progress_logger/trait.ProgressLog.html
[`ProgressLogger`]: https://docs.rs/dsi-progress-logger/latest/dsi_progress_logger/struct.ProgressLogger.html
[`start`]: https://docs.rs/dsi-progress-logger/latest/dsi_progress_logger/trait.ProgressLog.html#tymethod.start
[`item_name`]: https://docs.rs/dsi-progress-logger/latest/dsi_progress_logger/trait.ProgressLog.html#tymethod.item_name
[`log_interval`]: https://docs.rs/dsi-progress-logger/latest/dsi_progress_logger/trait.ProgressLog.html#tymethod.log_interval
[`expected_updates`]: https://docs.rs/dsi-progress-logger/latest/dsi_progress_logger/trait.ProgressLog.html#tymethod.expected_updates
[`refresh`]: https://docs.rs/dsi-progress-logger/latest/dsi_progress_logger/trait.ProgressLog.html#tymethod.refresh
[`stop`]: https://docs.rs/dsi-progress-logger/latest/dsi_progress_logger/trait.ProgressLog.html#tymethod.stop
[`done`]: https://docs.rs/dsi-progress-logger/latest/dsi_progress_logger/trait.ProgressLog.html#tymethod.done
[`info`]: https://docs.rs/dsi-progress-logger/latest/dsi_progress_logger/trait.ProgressLog.html#tymethod.info
[`clone`]: https://docs.rs/dsi-progress-logger/latest/dsi_progress_logger/trait.ProgressLog.html#tymethod.clone
[`display_memory`]: https://docs.rs/dsi-progress-logger/latest/dsi_progress_logger/trait.ProgressLog.html#tymethod.display_memory
[`update`]: https://docs.rs/dsi-progress-logger/latest/dsi_progress_logger/trait.ProgressLog.html#tymethod.light_update
[`light_update`]: https://docs.rs/dsi-progress-logger/latest/dsi_progress_logger/trait.ProgressLog.html#tymethod.light_update
[`LIGHT_UPDATE_MASK`]: https://docs.rs/dsi-progress-logger/latest/dsi_progress_logger/struct.ProgressLogger.html#associatedconstant.LIGHT_UPDATE_MASK
[`it.unimi.dsi.util.ProgressLogger`]: https://dsiutils.di.unimi.it/docs/it/unimi/dsi/logging/ProgressLogger.html
[DSI Utilities]: https://dsiutils.di.unimi.it/
[`log`]: https://docs.rs/log
[`Instant::now()`]: https://doc.rust-lang.org/std/time/struct.Instant.html#method.now
[`progress_logger`]: https://doc.rust-lang.org/std/time/struct.Instant.html#method.now
[`log_target`]: <https://docs.rs/dsi-progress-logger/latest/dsi_progress_logger/trait.ProgressLog.html#tymethod.log_target>
