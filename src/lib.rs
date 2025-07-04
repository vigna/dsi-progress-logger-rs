/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 * SPDX-FileCopyrightText: 2024 Fondation Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

#![doc = include_str!("../README.md")]

use log::{debug, error, info, trace, warn};
use num_format::{Locale, ToFormattedString};
use pluralizer::pluralize;
use std::fmt::{Arguments, Display, Formatter, Result};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use sysinfo::{MemoryRefreshKind, Pid, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};
mod utils;
pub use utils::*;

/// Logging trait.
///
/// To log the progress of an activity, you call [`start`](ProgressLog::start).
/// Then, each time you want to mark progress, you call
/// [`update`](ProgressLog::update), which increases the item counter, and will
/// log progress information if enough time has passed since the last log.
/// [`light_update`](ProgressLog::light_update) will perform a time check only
///  on a subset of updates (e.g., for [`ProgressLogger`], multiples of
/// [`LIGHT_UPDATE_MASK`](ProgressLogger::LIGHT_UPDATE_MASK) + 1); it  should be
/// used when the activity has an extremely low cost that is comparable to that
/// of the time check (a call to [`Instant::now()`]) itself.
///
/// A few setters can be called at any time to customize the logger (e.g.,
/// [`item_name`](ProgressLog::item_name),
/// [`log_interval`](ProgressLog::log_interval),
/// [`expected_updates`](ProgressLog::expected_updates), etc.). The setters take
/// and return a mutable reference to the logger, so you must first assign the
/// logger to a variable, and then you can chain-call the setters on the
/// variable in fluent style. The disadvantage of this approach is that you must
/// assign the logger to a variable, but the advantage is that you can call any
/// setter without having to reassign the variable holding the logger.
///
/// It is also possible to log used and free memory at each log interval by
/// calling [`display_memory`](ProgressLog::display_memory). Memory is read from
/// system data by the [`sysinfo`] crate, and will be updated at each log
/// interval (note that this will slightly slow down the logging process).
/// However, never use this feature in a
/// [`rayon`](https://crates.io/crates/rayon) environment if another crate in
/// your compilation unit depends on on
/// [`sysinfo`](https://crates.io/crates/sysinfo)'s (default) `multithread`
/// feature, as [this can lead to a
/// deadlock](https://github.com/rayon-rs/rayon/issues/592) .
///
///
/// At any time, displaying the progress logger will give you time information
/// up to the present. However,  since it is impossible to update the memory
/// information from the [`Display::fmt`] implementation, you should call
/// [`refresh`](ProgressLog::refresh) before displaying the logger on your own.
///
/// When the activity is over, you call [`stop`](ProgressLog::stop), which fixes
/// the final time, and possibly display again the logger.
///  [`done`](ProgressLog::done) will stop the logger, print `Completed.`, and
/// display the final stats.
///
/// After you finish a run of the progress logger, can call
/// [`start`](ProgressLog::start) again measure another activity.
///
/// As explained in the [crate documentation](crate), we suggest using `&mut
/// impl ProgressLog` to pass a logger as an argument, to be able to use
/// optional logging.
///
/// # Examples
///
/// See the [`ProgressLogger`] documentation.
pub trait ProgressLog {
    /// The type returned by [`concurrent`](ProgressLog::concurrent).
    type Concurrent: ConcurrentProgressLog;

    /// Force a log of `self` assuming `now` is the current time.
    ///
    /// This is a low-level method that should not be called directly.
    fn log(&mut self, now: Instant);

    /// Log `self` if it is time to log.
    ///
    /// This is a low-level method that should not be called directly.
    fn log_if(&mut self, now: Instant);

    /// Set the display of memory information.
    ///
    /// Memory information include:
    /// - the [resident-set size](sysinfo::Process::memory) of the process that
    ///   created the logger;
    /// - the [virtual-memory size](sysinfo::Process::virtual_memory) of the
    ///   process that created the logger;
    /// - the [available memory](sysinfo::System::available_memory);
    /// - the [free memory](`sysinfo::System::free_memory);
    /// - the [total amount](sysinfo::System::total_memory) of memory.
    ///
    /// Never use this feature in a [`rayon`](https://crates.io/crates/rayon)
    /// environment if another crate in your compilation unit depends on on
    /// [`sysinfo`](https://crates.io/crates/sysinfo)'s (default) `multithread`
    /// feature, as [this can lead to a
    /// deadlock](https://github.com/rayon-rs/rayon/issues/592) .
    fn display_memory(&mut self, display_memory: bool) -> &mut Self;

    /// Set the name of an item.
    fn item_name(&mut self, item_name: impl AsRef<str>) -> &mut Self;

    /// Set the log interval.
    fn log_interval(&mut self, log_interval: Duration) -> &mut Self;

    /// Set the expected number of updates.
    ///
    /// If not [`None`], the logger will display the percentage of completion
    /// and an estimate of the time to completion.
    fn expected_updates(&mut self, expected_updates: Option<usize>) -> &mut Self;

    /// Set the time unit to use for speed.
    ///
    /// If not [`None`], the logger will always display the speed in this unit
    /// instead of making a choice of readable unit based on the elapsed time.
    /// Moreover, large numbers will not be thousands separated. This behavior
    /// is useful when the output of the logger must be parsed.
    fn time_unit(&mut self, time_unit: Option<TimeUnit>) -> &mut Self;

    /// Set whether to display additionally the speed achieved during the last
    /// log interval.
    fn local_speed(&mut self, local_speed: bool) -> &mut Self;

    /// Set the [`log`] target.
    ///
    /// This should often be the path of the module logging progress, which is
    /// obtained with [`std::module_path!`].
    ///
    /// Note that the macro [`progress_logger!`] sets this field automatically
    /// to [`std::module_path!`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use dsi_progress_logger::prelude::*;
    ///
    ///    env_logger::builder()
    ///        .filter_level(log::LevelFilter::Info)
    ///        .try_init()?;
    ///
    /// let mut pl = ProgressLogger::default();
    /// pl.item_name("pumpkin");
    /// pl.log_target(std::module_path!());
    /// pl.start("Smashing pumpkins from a module...");
    /// for _ in 0..100 {
    ///    // do something on each pumpkin
    ///    pl.update();
    /// }
    /// pl.done();
    /// #     Ok(())
    /// # }
    /// ```
    fn log_target(&mut self, target: impl AsRef<str>) -> &mut Self;

    /// Add a value to the counter.
    ///
    /// This method is mainly useful for wrappers or to implement a custom
    /// update strategy.
    fn add_to_count(&mut self, count: usize);

    /// Start the logger, displaying the given message.
    ///
    /// You can pass the empty string to display nothing.
    fn start(&mut self, msg: impl AsRef<str>);

    /// Increase the count and check whether it is time to log.
    fn update(&mut self);

    /// Set the count and check whether it is time to log.
    fn update_with_count(&mut self, count: usize) {
        self.update_with_count_and_time(count, Instant::now());
    }

    /// Set the count and check whether it is time to log, given the current
    /// time.
    ///
    /// This method is mainly useful for wrappers that want to avoid unnecessary
    /// calls to [`Instant::now`].
    fn update_with_count_and_time(&mut self, count: usize, now: Instant);

    /// Increase the count but checks whether it is time to log only after an
    /// implementation-defined number of calls.
    ///
    /// Useful for very short activities with respect to which  checking the
    /// time is expensive.
    fn light_update(&mut self);

    /// Increase the count and forces a log.
    fn update_and_display(&mut self);

    /// Stop the logger, fixing the final time.
    fn stop(&mut self);

    /// Stop the logger, print `Completed.`, and display the final stats. The
    /// number of expected updates will be cleared.
    fn done(&mut self);

    /// Stop the logger, sets the count, prints `Completed.`, and displays the
    /// final stats. The number of expected updates will be cleared.
    ///
    /// This method is particularly useful in two circumstances:
    /// * you have updated the logger with some approximate values (e.g., in a
    ///   multicore computation) but before printing the final stats you want
    ///   the internal counter to contain an exact value;
    /// * you have used the logger as a handy timer, calling just
    ///   [`start`](ProgressLog::start) and this method.
    fn done_with_count(&mut self, count: usize);

    /// Return the elapsed time since the logger was started, or `None` if the
    /// logger has not been started.
    fn elapsed(&self) -> Option<Duration>;

    /// Return the last count the logger has been set to.
    ///
    /// Note that you can call this method even after the logger has been
    /// [stopped](ProgressLog::stop).
    fn count(&self) -> usize;

    /// Refresh memory information, if previously requested with
    /// [`display_memory`](#method.display_memory). You do not need to call this
    /// method unless you display the logger manually.
    fn refresh(&mut self);

    /// Output the given message at the [trace](`log::Level::Trace`) level.
    ///
    /// See [`info`](ProgressLog::info) for an example.
    fn trace(&self, args: Arguments<'_>);

    /// Output the given message at the [debug](`log::Level::Debug`) level.
    ///
    /// See [`info`](ProgressLog::info) for an example.
    fn debug(&self, args: Arguments<'_>);

    /// Output the given message at the [info](`log::Level::Info`) level.
    ///
    /// For maximum flexibility, this method takes as argument the result of a
    /// [`std::format_args!`] macro. Note that there will be no output if the
    /// logger is [`None`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use dsi_progress_logger::*;
    ///
    /// env_logger::builder()
    ///     .filter_level(log::LevelFilter::Info)
    ///     .try_init()?;
    ///
    /// let logger_name = "my_logger";
    /// let mut pl = progress_logger![];
    /// pl.info(format_args!("My logger named {}", logger_name));
    /// #     Ok(())
    /// # }
    /// ```
    fn info(&self, args: Arguments<'_>);

    /// Output the given message at the [warn](`log::Level::Warn`) level.
    ///
    /// See [`info`](ProgressLog::info) for an example.
    fn warn(&self, args: Arguments<'_>);

    /// Output the given message at the [error](`log::Level::Error`) level.
    ///
    /// See [`info`](ProgressLog::info) for an example.
    fn error(&self, args: Arguments<'_>);

    /// Return a concurrent copy of the logger.
    ///
    /// Some methods require both sequential and concurrent logging. To keep
    /// optional logging efficient, we suggest in this cases to use `&mut impl
    /// ProgressLog` to pass a logger as an argument, and then creating a
    /// concurrent copy of the logger with this method. If the original logger
    /// is `None`, the concurrent copy will be `None` as well.
    ///
    /// Note that the result of the method is a copy—it will not share the state
    /// of the original logger.
    ///
    /// Concurrent logger implementations can just return a duplicate of
    /// themselves. [`dup`](ConcurrentProgressLog::dup).
    fn concurrent(&self) -> Self::Concurrent;
}

/// Concurrent logging trait.
///
/// This trait extends [`ProgressLog`] by adding a
/// [`dup`](ConcurrentProgressLog::dup) method that duplicates the logger and
/// adding the [`Clone`], [`Sync`], and [`Send`] traits.
///
/// By contract, [`Clone`] implementations must return a new logger updating the
/// same internal state, so you can easily use a [`ConcurrentProgressLog`] in
/// methods like
/// [`rayon::ParallelIterator::for_each_with`](https://docs.rs/rayon/latest/rayon/iter/trait.ParallelIterator.html#method.for_each_with),
/// [`rayon::ParallelIterator::map_with`](https://docs.rs/rayon/latest/rayon/iter/trait.ParallelIterator.html#method.map_with),
/// and so on. In a [`rayon`](https://docs.rs/rayon) environment, however, you
/// cannot use [`display_memory`](ProgressLog::display_memory) if another crate
/// in your compilation unit depends on on
/// [`sysinfo`](https://crates.io/crates/sysinfo)'s (default) `multithread`
/// feature, as [this can lead to a
/// deadlock](https://github.com/rayon-rs/rayon/issues/592) .
///
/// Note that [`ProgressLogger`]'s [`Clone`
/// implementation](ProgressLogger#impl-Clone-for-ProgressLogger) has a
/// completely different semantics.
///
/// As explained in the [crate documentation](crate), we suggest using `&mut
/// Self::Concurrent` to pass a concurrent logger as an argument, to be able to
/// use optional logging.
///
/// # Examples
///
/// See the [`ConcurrentWrapper`] documentation. type Concurrent =
///     Option<P::Concurrent>;
///
pub trait ConcurrentProgressLog: ProgressLog + Sync + Send + Clone {
    /// The type returned by [`dup`](ConcurrentProgressLog::dup).
    type Duplicated: ConcurrentProgressLog;

    /// Duplicate the concurrent progress logger, obtaining a new one.
    ///
    /// Note that the this method has the same semantics of [`ProgressLogger`'s
    /// `Clone` implementation](ProgressLogger#impl-Clone-for-ProgressLogger),
    /// but in a [`ConcurrentProgressLog`] by contract [cloning must generate
    /// copies with the same underlying logger](ConcurrentProgressLog).
    fn dup(&self) -> Self::Duplicated;
}

impl<P: ProgressLog> ProgressLog for &mut P {
    type Concurrent = P::Concurrent;

    fn log(&mut self, now: Instant) {
        (**self).log(now);
    }

    fn log_if(&mut self, now: Instant) {
        (**self).log_if(now);
    }

    fn add_to_count(&mut self, count: usize) {
        (**self).add_to_count(count);
    }

    fn display_memory(&mut self, display_memory: bool) -> &mut Self {
        (**self).display_memory(display_memory);
        self
    }

    fn item_name(&mut self, item_name: impl AsRef<str>) -> &mut Self {
        (**self).item_name(item_name);
        self
    }

    fn log_interval(&mut self, log_interval: Duration) -> &mut Self {
        (**self).log_interval(log_interval);
        self
    }

    fn expected_updates(&mut self, expected_updates: Option<usize>) -> &mut Self {
        (**self).expected_updates(expected_updates);
        self
    }

    fn time_unit(&mut self, time_unit: Option<TimeUnit>) -> &mut Self {
        (**self).time_unit(time_unit);
        self
    }

    fn local_speed(&mut self, local_speed: bool) -> &mut Self {
        (**self).local_speed(local_speed);
        self
    }

    fn log_target(&mut self, target: impl AsRef<str>) -> &mut Self {
        (**self).log_target(target);
        self
    }

    fn start(&mut self, msg: impl AsRef<str>) {
        (**self).start(msg);
    }

    fn update(&mut self) {
        (**self).update();
    }

    fn update_with_count_and_time(&mut self, count: usize, now: Instant) {
        (**self).update_with_count_and_time(count, now);
    }

    fn light_update(&mut self) {
        (**self).light_update();
    }

    fn update_and_display(&mut self) {
        (**self).update_and_display();
    }

    fn stop(&mut self) {
        (**self).stop();
    }

    fn done(&mut self) {
        (**self).done();
    }

    fn done_with_count(&mut self, count: usize) {
        (**self).done_with_count(count);
    }

    fn elapsed(&self) -> Option<Duration> {
        (**self).elapsed()
    }

    fn count(&self) -> usize {
        (**self).count()
    }

    fn refresh(&mut self) {
        (**self).refresh();
    }

    fn trace(&self, args: Arguments<'_>) {
        (**self).trace(args);
    }

    fn debug(&self, args: Arguments<'_>) {
        (**self).debug(args);
    }

    fn info(&self, args: Arguments<'_>) {
        (**self).info(args);
    }

    fn warn(&self, args: Arguments<'_>) {
        (**self).warn(args);
    }

    fn error(&self, args: Arguments<'_>) {
        (**self).error(args);
    }

    fn concurrent(&self) -> Self::Concurrent {
        (**self).concurrent()
    }
}

impl<P: ProgressLog> ProgressLog for Option<P> {
    type Concurrent = Option<P::Concurrent>;

    fn log(&mut self, now: Instant) {
        if let Some(pl) = self {
            pl.log(now);
        }
    }

    fn log_if(&mut self, now: Instant) {
        if let Some(pl) = self {
            pl.log_if(now);
        }
    }

    fn add_to_count(&mut self, count: usize) {
        if let Some(pl) = self {
            pl.add_to_count(count);
        }
    }

    fn display_memory(&mut self, display_memory: bool) -> &mut Self {
        if let Some(pl) = self {
            pl.display_memory(display_memory);
        }
        self
    }

    fn item_name(&mut self, item_name: impl AsRef<str>) -> &mut Self {
        if let Some(pl) = self {
            pl.item_name(item_name);
        }
        self
    }

    fn log_interval(&mut self, log_interval: Duration) -> &mut Self {
        if let Some(pl) = self {
            pl.log_interval(log_interval);
        }
        self
    }

    fn expected_updates(&mut self, expected_updates: Option<usize>) -> &mut Self {
        if let Some(pl) = self {
            pl.expected_updates(expected_updates);
        }
        self
    }

    fn time_unit(&mut self, time_unit: Option<TimeUnit>) -> &mut Self {
        if let Some(pl) = self {
            pl.time_unit(time_unit);
        }
        self
    }

    /// Sets whether to display additionally the speed achieved during the last
    /// log interval.
    fn local_speed(&mut self, local_speed: bool) -> &mut Self {
        if let Some(pl) = self {
            pl.local_speed(local_speed);
        }
        self
    }

    fn log_target(&mut self, target: impl AsRef<str>) -> &mut Self {
        if let Some(pl) = self {
            pl.log_target(target);
        }
        self
    }

    fn start(&mut self, msg: impl AsRef<str>) {
        if let Some(pl) = self {
            pl.start(msg);
        }
    }

    fn update(&mut self) {
        if let Some(pl) = self {
            pl.update();
        }
    }

    fn update_with_count_and_time(&mut self, count: usize, now: Instant) {
        if let Some(pl) = self {
            pl.update_with_count_and_time(count, now);
        }
    }

    fn light_update(&mut self) {
        if let Some(pl) = self {
            pl.light_update();
        }
    }

    fn update_and_display(&mut self) {
        if let Some(pl) = self {
            pl.update_and_display();
        }
    }

    fn stop(&mut self) {
        if let Some(pl) = self {
            pl.stop();
        }
    }

    fn done(&mut self) {
        if let Some(pl) = self {
            pl.done();
        }
    }

    fn done_with_count(&mut self, count: usize) {
        if let Some(pl) = self {
            pl.done_with_count(count);
        }
    }

    fn elapsed(&self) -> Option<Duration> {
        self.as_ref().and_then(|pl| pl.elapsed())
    }

    fn count(&self) -> usize {
        self.as_ref().map(|pl| pl.count()).unwrap_or(0)
    }

    fn refresh(&mut self) {
        if let Some(pl) = self {
            pl.refresh();
        }
    }

    fn trace(&self, args: Arguments<'_>) {
        if let Some(pl) = self {
            pl.trace(args);
        }
    }

    fn debug(&self, args: Arguments<'_>) {
        if let Some(pl) = self {
            pl.debug(args);
        }
    }

    fn info(&self, args: Arguments<'_>) {
        if let Some(pl) = self {
            pl.info(args);
        }
    }

    fn warn(&self, args: Arguments<'_>) {
        if let Some(pl) = self {
            pl.warn(args);
        }
    }

    fn error(&self, args: Arguments<'_>) {
        if let Some(pl) = self {
            pl.error(args);
        }
    }

    fn concurrent(&self) -> Self::Concurrent {
        self.as_ref().map(|pl| pl.concurrent())
    }
}

impl<P: ConcurrentProgressLog> ConcurrentProgressLog for Option<P> {
    type Duplicated = Option<P::Duplicated>;

    fn dup(&self) -> Self::Duplicated {
        self.as_ref().map(|pl| pl.dup())
    }
}

/// An implementation of [`ProgressLog`] with output generated using the
/// [`log`](https://docs.rs/log) crate at the `info` level.
///
/// Instances can be created by using fluent setters, or by using the
/// [`progress_logger`] macro.
///
/// You can [clone](#impl-Clone-for-ProgressLogger) a logger to create a new one
/// with the same setup but with all the counters reset. This behavior is useful
/// when you want to configure a logger and then use its configuration for other
/// loggers.
///
/// # Examples
///
/// A typical call sequence to a progress logger is as follows:
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use dsi_progress_logger::prelude::*;
///
/// env_logger::builder().filter_level(log::LevelFilter::Info).try_init()?;
///
/// let mut pl = ProgressLogger::default();
/// pl.item_name("pumpkin");
/// pl.start("Smashing pumpkins...");
/// for _ in 0..100 {
///    // do something on each pumpkin
///    pl.update();
/// }
/// pl.done();
/// #     Ok(())
/// # }
/// ```
///
/// The [`progress_logger`] macro will create the progress logger for you and
/// set its [`log_target`](ProgressLog::log_target) to [`std::module_path!()`],
/// which is usually what you want. You can also call any setter with a
/// key-value syntax:
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use dsi_progress_logger::prelude::*;
///
/// env_logger::builder().filter_level(log::LevelFilter::Info).try_init()?;
///
/// let mut pl = progress_logger![item_name="pumpkin"];
/// pl.start("Smashing pumpkins...");
/// for _ in 0..100 {
///    // do something on each pumpkin
///    pl.update();
/// }
/// pl.done();
/// #     Ok(())
/// # }
/// ```
///
/// A progress logger can also be used as a handy timer:
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use dsi_progress_logger::prelude::*;
///
/// env_logger::builder().filter_level(log::LevelFilter::Info).try_init()?;
///
/// let mut pl = progress_logger![item_name="pumpkin"];
/// pl.start("Smashing pumpkins...");
/// for _ in 0..100 {
///    // do something on each pumpkin
/// }
/// pl.done_with_count(100);
/// #     Ok(())
/// # }
/// ```
///
/// This progress logger will display information about  memory usage:
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use dsi_progress_logger::prelude::*;
///
/// env_logger::builder().filter_level(log::LevelFilter::Info).try_init()?;
///
/// let mut pl = progress_logger![display_memory=true];
/// #     Ok(())
/// # }
/// ```
pub struct ProgressLogger {
    /// The name of an item. Defaults to `item`.
    item_name: String,
    /// The pluralized name of an item. Defaults to `items`. It is quite
    /// expensive to compute with [`pluralize`](pluralizer::pluralize), hence
    /// the caching.
    items_name: String,
    /// The log interval. Defaults to 10 seconds.
    log_interval: Duration,
    /// The expected number of updates. If set, the logger will display the percentage of completion and
    /// an estimate of the time to completion.
    expected_updates: Option<usize>,
    /// The time unit to use for speed. If set, the logger will always display the speed in this unit
    /// instead of making a choice of readable unit based on the elapsed time. Moreover, large numbers
    /// will not be thousands separated. This is useful when the output of the logger must be parsed.
    time_unit: Option<TimeUnit>,
    /// Display additionally the speed achieved during the last log interval.
    local_speed: bool,
    /// [`log`] target
    ///
    /// This is often the path of the module logging progress.
    log_target: String,
    /// When the logger was started.
    start_time: Option<Instant>,
    /// The last time we logged the activity (to compute speed).
    last_log_time: Instant,
    /// The next time we will log the activity.
    next_log_time: Instant,
    /// When the logger was stopped.
    stop_time: Option<Instant>,
    /// The number of items.
    count: usize,
    /// The number of items at the last log (to compute speed).
    last_count: usize,
    /// Display additionally the amount of used and free memory using this [`sysinfo::System`]
    system: Option<System>,
    /// The pid of the current process
    pid: Pid,
}

impl Default for ProgressLogger {
    /// Create a default [`ProgressLogger`] with a log interval of 10 seconds and
    /// item name set to “item”.
    fn default() -> Self {
        Self {
            item_name: "item".into(),
            items_name: "items".into(),
            log_interval: Duration::from_secs(10),
            expected_updates: None,
            time_unit: None,
            local_speed: false,
            log_target: std::env::current_exe()
                .ok()
                .and_then(|path| {
                    path.file_name()
                        .and_then(|s| s.to_owned().into_string().ok())
                })
                .unwrap_or_else(|| "main".to_string()),
            start_time: None,
            last_log_time: Instant::now(),
            next_log_time: Instant::now(),
            stop_time: None,
            count: 0,
            last_count: 0,
            system: None,
            pid: Pid::from(std::process::id() as usize),
        }
    }
}

impl Clone for ProgressLogger {
    /// Clone the logger, returning a logger with the same setup but with all the
    /// counters reset.
    #[allow(clippy::manual_map)]
    fn clone(&self) -> Self {
        Self {
            item_name: self.item_name.clone(),
            items_name: self.items_name.clone(),
            log_interval: self.log_interval,
            time_unit: self.time_unit,
            local_speed: self.local_speed,
            system: match self.system {
                Some(_) => Some(System::new_with_specifics(
                    RefreshKind::nothing().with_memory(MemoryRefreshKind::nothing().with_ram()),
                )),
                None => None,
            },
            ..ProgressLogger::default()
        }
    }
}

/// Macro to create a [`ProgressLogger`] with default log target set to
/// [`std::module_path!`], and key-value pairs instead of setters.
///
/// # Examples
///
///
/// ```rust
/// use dsi_progress_logger::prelude::*;
///
/// let mut pl = progress_logger![item_name="pumpkin", display_memory=true];
/// ```
#[macro_export]
macro_rules! progress_logger {
    ($($method:ident = $arg:expr),* $(,)?) => {
        {
            let mut pl = ::dsi_progress_logger::ProgressLogger::default();
            ::dsi_progress_logger::ProgressLog::log_target(&mut pl, ::std::module_path!());
            $(
                ::dsi_progress_logger::ProgressLog::$method(&mut pl, $arg);
            )*
            pl
        }
    }
}

impl ProgressLogger {
    /// Calls to [light_update](ProgressLog::light_update) will cause a call to
    /// [`Instant::now`] only if the current count is a multiple of this mask
    /// plus one.
    pub const LIGHT_UPDATE_MASK: usize = (1 << 20) - 1;

    fn fmt_timing_speed(&self, f: &mut Formatter<'_>, seconds_per_item: f64) -> Result {
        let items_per_second = 1.0 / seconds_per_item;

        let time_unit_timing = self
            .time_unit
            .unwrap_or_else(|| TimeUnit::nice_time_unit(seconds_per_item));

        let time_unit_speed = self
            .time_unit
            .unwrap_or_else(|| TimeUnit::nice_speed_unit(seconds_per_item));

        f.write_fmt(format_args!(
            "{:.2} {}/{}, {:.2} {}/{}",
            items_per_second * time_unit_speed.as_seconds(),
            self.items_name,
            time_unit_speed.label(),
            seconds_per_item / time_unit_timing.as_seconds(),
            time_unit_timing.label(),
            self.item_name
        ))?;

        Ok(())
    }
}

impl ProgressLog for ProgressLogger {
    type Concurrent = ConcurrentWrapper<Self>;

    fn log(&mut self, now: Instant) {
        self.refresh();
        info!(target: &self.log_target, "{}", self);
        self.last_count = self.count;
        self.last_log_time = now;
        self.next_log_time = now + self.log_interval;
    }

    fn log_if(&mut self, now: Instant) {
        if self.next_log_time <= now {
            self.log(now);
        }
    }

    fn add_to_count(&mut self, count: usize) {
        self.count += count;
    }

    fn display_memory(&mut self, display_memory: bool) -> &mut Self {
        match (display_memory, &self.system) {
            (true, None) => {
                self.system = Some(System::new_with_specifics(
                    RefreshKind::nothing().with_memory(MemoryRefreshKind::nothing().with_ram()),
                ));
            }
            (false, Some(_)) => {
                self.system = None;
            }
            _ => (),
        }
        self
    }

    fn item_name(&mut self, item_name: impl AsRef<str>) -> &mut Self {
        self.item_name = item_name.as_ref().into();
        self.items_name = pluralize(item_name.as_ref(), 2, false);
        self
    }

    fn log_interval(&mut self, log_interval: Duration) -> &mut Self {
        self.log_interval = log_interval;
        self
    }

    fn expected_updates(&mut self, expected_updates: Option<usize>) -> &mut Self {
        self.expected_updates = expected_updates;
        self
    }

    fn time_unit(&mut self, time_unit: Option<TimeUnit>) -> &mut Self {
        self.time_unit = time_unit;
        self
    }

    fn local_speed(&mut self, local_speed: bool) -> &mut Self {
        self.local_speed = local_speed;
        self
    }

    fn log_target(&mut self, target: impl AsRef<str>) -> &mut Self {
        self.log_target = target.as_ref().into();
        self
    }

    fn start(&mut self, msg: impl AsRef<str>) {
        let now = Instant::now();
        self.start_time = Some(now);
        self.stop_time = None;
        self.count = 0;
        self.last_count = 0;
        self.last_log_time = now;
        self.next_log_time = now + self.log_interval;
        if !msg.as_ref().is_empty() {
            info!(target: &self.log_target, "{}", msg.as_ref());
        }
    }

    fn refresh(&mut self) {
        if let Some(system) = &mut self.system {
            system.refresh_processes_specifics(
                ProcessesToUpdate::Some(&[self.pid]),
                false,
                ProcessRefreshKind::nothing().with_memory(),
            );
        }
    }

    fn update(&mut self) {
        self.count += 1;
        self.log_if(Instant::now());
    }

    fn update_with_count_and_time(&mut self, count: usize, now: Instant) {
        self.count += count;
        self.log_if(now);
    }

    /// Increases the count and, once every
    /// [`LIGHT_UPDATE_MASK`](#fields.LIGHT_UPDATE_MASK) + 1 calls, check
    /// whether it is time to log.
    #[inline(always)]
    fn light_update(&mut self) {
        self.count += 1;
        if (self.count & Self::LIGHT_UPDATE_MASK) == 0 {
            self.log_if(Instant::now());
        }
    }

    fn update_and_display(&mut self) {
        self.count += 1;
        self.log(Instant::now());
    }

    fn stop(&mut self) {
        self.stop_time = Some(Instant::now());
        self.expected_updates = None;
    }

    fn done(&mut self) {
        self.stop();
        info!(target: &self.log_target, "Completed.");
        // just to avoid wrong reuses
        self.expected_updates = None;
        self.refresh();
        info!(target: &self.log_target, "{}", self);
    }

    fn done_with_count(&mut self, count: usize) {
        self.count = count;
        self.done();
    }

    fn elapsed(&self) -> Option<Duration> {
        self.start_time?.elapsed().into()
    }

    fn count(&self) -> usize {
        self.count
    }

    fn trace(&self, args: Arguments<'_>) {
        trace!(target: &self.log_target, "{}", std::fmt::format(args));
    }

    fn debug(&self, args: Arguments<'_>) {
        debug!(target: &self.log_target, "{}", std::fmt::format(args));
    }

    fn info(&self, args: Arguments<'_>) {
        info!(target: &self.log_target, "{}", std::fmt::format(args));
    }

    fn warn(&self, args: Arguments<'_>) {
        warn!(target: &self.log_target, "{}", std::fmt::format(args));
    }

    fn error(&self, args: Arguments<'_>) {
        error!(target: &self.log_target, "{}", std::fmt::format(args));
    }

    fn concurrent(&self) -> Self::Concurrent {
        ConcurrentWrapper::wrap(self.clone())
    }
}

impl Display for ProgressLogger {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        if let Some(start_time) = self.start_time {
            let count_fmtd = if self.time_unit.is_none() {
                self.count.to_formatted_string(&Locale::en)
            } else {
                self.count.to_string()
            };

            if let Some(stop_time) = self.stop_time {
                let elapsed = stop_time - start_time;
                let seconds_per_item = elapsed.as_secs_f64() / self.count as f64;

                f.write_fmt(format_args!(
                    "Elapsed: {}",
                    TimeUnit::pretty_print(elapsed.as_millis())
                ))?;

                if self.count != 0 {
                    f.write_fmt(format_args!(
                        " [{} {}, ",
                        count_fmtd,
                        if self.count == 1 {
                            &self.item_name
                        } else {
                            &self.items_name
                        }
                    ))?;
                    self.fmt_timing_speed(f, seconds_per_item)?;
                    f.write_fmt(format_args!("]"))?
                }
            } else {
                let now = Instant::now();

                let elapsed = now - start_time;

                f.write_fmt(format_args!(
                    "{} {}, {}, ",
                    count_fmtd,
                    if self.count == 1 {
                        &self.item_name
                    } else {
                        &self.items_name
                    },
                    TimeUnit::pretty_print(elapsed.as_millis()),
                ))?;

                let seconds_per_item = elapsed.as_secs_f64() / self.count as f64;
                self.fmt_timing_speed(f, seconds_per_item)?;

                if let Some(expected_updates) = self.expected_updates {
                    let millis_to_end: u128 = (expected_updates.saturating_sub(self.count) as u128
                        * elapsed.as_millis())
                        / (self.count as u128 + 1);
                    f.write_fmt(format_args!(
                        "; {:.2}% done, {} to end",
                        100.0 * self.count as f64 / expected_updates as f64,
                        TimeUnit::pretty_print(millis_to_end)
                    ))?;
                }

                if self.local_speed && self.stop_time.is_none() {
                    f.write_fmt(format_args!(" ["))?;

                    let elapsed = now - self.last_log_time;
                    let seconds_per_item =
                        elapsed.as_secs_f64() / (self.count - self.last_count) as f64;
                    self.fmt_timing_speed(f, seconds_per_item)?;

                    f.write_fmt(format_args!("]"))?;
                }
            }

            // It would be ideal to refresh self.system here, but this operation
            // would require an &mut self reference.
            if let Some(system) = &self.system {
                f.write_fmt(format_args!(
                    "; res/vir/avail/free/total mem {}/{}/{}B/{}B/{}B",
                    system
                        .process(self.pid)
                        .map(|process| humanize(process.memory() as _) + "B")
                        .unwrap_or("N/A".to_string()),
                    system
                        .process(self.pid)
                        .map(|process| humanize(process.virtual_memory() as _) + "B")
                        .unwrap_or("N/A".to_string()),
                    humanize(system.available_memory() as _),
                    humanize(system.free_memory() as _),
                    humanize(system.total_memory() as _)
                ))?;
            }

            Ok(())
        } else {
            write!(f, "ProgressLogger not started")
        }
    }
}

/// A [`ConcurrentProgressLog`] implementation that wraps a [`ProgressLog`] in
/// an [`Arc`]/[`Mutex`].
///
/// The methods [`update`](ProgressLog::update) and
/// [`update_with_count`](ProgressLog::update_with_count) buffer the increment
/// and add it to the underlying logger only when the buffer reaches a
/// threshold; this prevents locking the underlying logger too often. The
/// threshold is set at creation using the methods
/// [`with_threshold`](Self::with_threshold) and
/// [`wrap_with_threshold`](Self::wrap_with_threshold), or by calling the method
/// [`threshold`](Self::threshold).
///
/// The method [`light_update`](ProgressLog::light_update), as in the case of
/// [`ProgressLogger`], further delays updates using an even faster check.
///
/// # Examples
///
/// In this example, we manually spawn processes:
///
/// ```rust
/// use dsi_progress_logger::prelude::*;
/// use std::thread;
///
/// let mut cpl = concurrent_progress_logger![item_name = "pumpkin"];
/// cpl.start("Smashing pumpkins (using many threads)...");
///
/// std::thread::scope(|s| {
///     for i in 0..100 {
///         let mut pl = cpl.clone();
///         s.spawn(move || {
///             for _ in 0..100000 {
///                 // do something on each pumpkin
///                 pl.update();
///             }
///         });
///     }
/// });
///
/// cpl.done();
/// ```
///
/// You can obtain the same behavior with
/// [`rayon`](https://crates.io/crates/rayon) using methods such as
/// [`for_each_with`](https://docs.rs/rayon/latest/rayon/iter/trait.ParallelIterator.html#method.for_each_with)
/// and
/// [`map_with`](https://docs.rs/rayon/latest/rayon/iter/trait.ParallelIterator.html#method.map_with):
///
/// ```rust
/// use dsi_progress_logger::prelude::*;
/// use rayon::prelude::*;
///
/// let mut cpl = concurrent_progress_logger![item_name = "pumpkin"];
/// cpl.start("Smashing pumpkins (using many threads)...");
///
/// (0..1000000).into_par_iter().
///     with_min_len(1000). // optional, might reduce the amount of cloning
///     for_each_with(cpl.clone(), |pl, i| {
///         // do something on each pumpkin
///         pl.update();
///     }
/// );
///
/// cpl.done();
/// ```
///
/// Note that you have to pass `cpl.clone()` to avoid a move that would make the
/// call to [`done`](ProgressLog::done) impossible. Also, since
/// [`for_each_with`](https://docs.rs/rayon/latest/rayon/iter/trait.ParallelIterator.html#method.for_each_with)
/// might perform excessive cloning if jobs are too short, you can use
/// [`with_min_len`](https://docs.rs/rayon/latest/rayon/iter/trait.ParallelIterator.html#method.with_min_len)
/// to reduce the amount of cloning.
pub struct ConcurrentWrapper<P: ProgressLog = ProgressLogger> {
    /// Underlying logger
    inner: Arc<Mutex<P>>,
    /// The number of items processed by the current thread.
    local_count: u32,
    /// The threshold for updating the underlying logger.
    threshold: u32,
}

impl Default for ConcurrentWrapper {
    /// Create a new [`ConcurrentWrapper`] based on a default
    /// [`ProgressLogger`], with a threshold of
    /// [`DEFAULT_THRESHOLD`](Self::DEFAULT_THRESHOLD).
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(ProgressLogger::default())),
            local_count: 0,
            threshold: Self::DEFAULT_THRESHOLD,
        }
    }
}

impl<P: ProgressLog + Clone> Clone for ConcurrentWrapper<P> {
    /// Clone the concurrent wrapper, obtaining a new one with the same
    /// threshold, with a local count of zero, and with the same inner
    /// [`ProgressLog`].
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            local_count: 0,
            threshold: self.threshold,
        }
    }
}

/// Macro to create a [`ConcurrentWrapper`] based on a
/// [`ProgressLogger`], with default log target set to [`std::module_path!`],
/// and key-value pairs instead of setters.
///
/// # Examples
///
/// ```rust
/// use dsi_progress_logger::prelude::*;
///
/// let mut pl = concurrent_progress_logger![item_name="pumpkin", display_memory=true];
/// ```
#[macro_export]
macro_rules! concurrent_progress_logger {
    ($($method:ident = $arg:expr),* $(,)?) => {
        {
            let mut cpl = ::dsi_progress_logger::ConcurrentWrapper::default();
            ::dsi_progress_logger::ProgressLog::log_target(&mut cpl, ::std::module_path!());
            $(
                ::dsi_progress_logger::ProgressLog::$method(&mut cpl, $arg);
            )*
            cpl
        }
    }
}

impl ConcurrentWrapper {
    /// Create a new [`ConcurrentWrapper`] based on a default
    /// [`ProgressLogger`], using the [default
    /// threshold](Self::DEFAULT_THRESHOLD).
    pub fn new() -> Self {
        Self::with_threshold(Self::DEFAULT_THRESHOLD)
    }

    /// Create a new [`ConcurrentWrapper`] wrapping a default
    /// [`ProgressLogger`], using the given threshold.
    pub fn with_threshold(threshold: u32) -> Self {
        Self {
            inner: Arc::new(Mutex::new(ProgressLogger::default())),
            local_count: 0,
            threshold,
        }
    }
}

impl<P: ProgressLog> ConcurrentWrapper<P> {
    /// The default threshold for updating the underlying logger.
    pub const DEFAULT_THRESHOLD: u32 = 1 << 15;

    /// Calls to [`light_update`](ProgressLog::light_update) will cause a call
    /// to [`update_with_count`](ProgressLog::update_with_count) only if the
    /// current local count is a multiple of this mask plus one.
    ///
    /// Note that this constant is significantly smaller than the one used in
    /// [`ProgressLogger`], as updates will be further delayed by the threshold
    /// mechanism.
    pub const LIGHT_UPDATE_MASK: u32 = (1 << 10) - 1;

    /// Set the threshold for updating the underlying logger.
    ///
    /// Note that concurrent loggers with the same underlying logger
    /// have independent thresholds.
    pub fn threshold(&mut self, threshold: u32) -> &mut Self {
        self.threshold = threshold;
        self
    }

    /// Wrap a given [`ProgressLog`] in a [`ConcurrentWrapper`]
    /// using the [default threshold](Self::DEFAULT_THRESHOLD).
    pub fn wrap(inner: P) -> Self {
        Self {
            inner: Arc::new(Mutex::new(inner)),
            local_count: 0,
            threshold: Self::DEFAULT_THRESHOLD,
        }
    }

    /// Wrap a given [`ProgressLog`] in a [`ConcurrentWrapper`] using a
    /// given threshold.
    pub fn wrap_with_threshold(inner: P, threshold: u32) -> Self {
        Self {
            inner: Arc::new(Mutex::new(inner)),
            local_count: 0,
            threshold,
        }
    }

    /// Force an update of the underlying logger with the current local count.
    pub fn flush(&mut self) {
        self.inner
            .lock()
            .unwrap()
            .update_with_count(self.local_count as _);
        self.local_count = 0;
    }
}

impl<P: ProgressLog + Clone> ConcurrentWrapper<P> {
    /// Duplicates the concurrent wrapper, obtaining a new one with the same
    /// threshold, with a local count of zero, and with an inner
    /// [`ProgressLog`] that is a clone of the original one.
    pub fn dup(&self) -> Self {
        Self {
            inner: Arc::new(Mutex::new(self.inner.lock().unwrap().clone())),
            local_count: 0,
            threshold: self.threshold,
        }
    }
}

impl<P: ProgressLog + Clone + Send> ConcurrentProgressLog for ConcurrentWrapper<P> {
    type Duplicated = ConcurrentWrapper<P>;
    fn dup(&self) -> Self {
        Self {
            inner: Arc::new(Mutex::new(self.inner.lock().unwrap().clone())),
            local_count: 0,
            threshold: self.threshold,
        }
    }
}

impl<P: ProgressLog + Clone + Send> ProgressLog for ConcurrentWrapper<P> {
    type Concurrent = Self;

    fn log(&mut self, now: Instant) {
        self.inner.lock().unwrap().log(now);
    }

    fn log_if(&mut self, now: Instant) {
        self.inner.lock().unwrap().log_if(now);
    }

    fn add_to_count(&mut self, count: usize) {
        self.inner.lock().unwrap().add_to_count(count);
    }

    fn display_memory(&mut self, display_memory: bool) -> &mut Self {
        self.inner.lock().unwrap().display_memory(display_memory);
        self
    }

    fn item_name(&mut self, item_name: impl AsRef<str>) -> &mut Self {
        self.inner.lock().unwrap().item_name(item_name);
        self
    }

    fn log_interval(&mut self, log_interval: Duration) -> &mut Self {
        self.inner.lock().unwrap().log_interval(log_interval);
        self
    }

    fn expected_updates(&mut self, expected_updates: Option<usize>) -> &mut Self {
        self.inner
            .lock()
            .unwrap()
            .expected_updates(expected_updates);
        self
    }

    fn time_unit(&mut self, time_unit: Option<TimeUnit>) -> &mut Self {
        self.inner.lock().unwrap().time_unit(time_unit);
        self
    }

    fn local_speed(&mut self, local_speed: bool) -> &mut Self {
        self.inner.lock().unwrap().local_speed(local_speed);
        self
    }

    fn log_target(&mut self, target: impl AsRef<str>) -> &mut Self {
        self.inner.lock().unwrap().log_target(target);
        self
    }

    fn start(&mut self, msg: impl AsRef<str>) {
        self.inner.lock().unwrap().start(msg);
        self.local_count = 0;
    }

    #[inline]
    fn update(&mut self) {
        self.update_with_count(1)
    }

    #[inline]
    fn update_with_count_and_time(&mut self, count: usize, _now: Instant) {
        self.update_with_count(count);
    }

    #[inline]
    fn update_with_count(&mut self, count: usize) {
        match (self.local_count as usize).checked_add(count) {
            None => {
                // Sum overflows, update in two steps
                {
                    let now = Instant::now();
                    let mut inner = self.inner.lock().unwrap();
                    inner.update_with_count_and_time(self.local_count as _, now);
                    inner.update_with_count_and_time(count, now);
                }
                self.local_count = 0;
            }
            Some(total_count) => {
                if total_count >= self.threshold as usize {
                    self.local_count = 0;
                    // Threshold reached, time to flush to the inner ProgressLog
                    let now = Instant::now();
                    self.inner
                        .lock()
                        .unwrap()
                        .update_with_count_and_time(total_count, now);
                } else {
                    // total_count is lower than self.threshold, which is a u32;
                    // so total_count fits in u32.
                    self.local_count = total_count as u32;
                }
            }
        }
    }

    #[inline]
    fn light_update(&mut self) {
        self.local_count += 1;
        if (self.local_count & Self::LIGHT_UPDATE_MASK) == 0 && self.local_count >= self.threshold {
            // Threshold reached, time to flush to the inner ProgressLog
            let local_count = self.local_count as usize;
            self.local_count = 0;
            let now = Instant::now();
            self.inner
                .lock()
                .unwrap()
                .update_with_count_and_time(local_count, now);
        }
    }

    fn update_and_display(&mut self) {
        {
            let mut inner = self.inner.lock().unwrap();
            inner.add_to_count(self.local_count as _);
            inner.update_and_display();
        }
        self.local_count = 0;
    }

    fn stop(&mut self) {
        self.inner.lock().unwrap().stop();
    }

    fn done(&mut self) {
        self.inner.lock().unwrap().done();
    }

    fn done_with_count(&mut self, count: usize) {
        self.inner.lock().unwrap().done_with_count(count);
    }

    fn elapsed(&self) -> Option<Duration> {
        self.inner.lock().unwrap().elapsed()
    }

    fn count(&self) -> usize {
        self.inner.lock().unwrap().count()
    }

    fn refresh(&mut self) {
        self.inner.lock().unwrap().refresh();
    }

    fn trace(&self, args: Arguments<'_>) {
        self.inner.lock().unwrap().trace(args);
    }

    fn debug(&self, args: Arguments<'_>) {
        self.inner.lock().unwrap().debug(args);
    }

    fn info(&self, args: Arguments<'_>) {
        self.inner.lock().unwrap().info(args);
    }

    fn warn(&self, args: Arguments<'_>) {
        self.inner.lock().unwrap().warn(args);
    }

    fn error(&self, args: Arguments<'_>) {
        self.inner.lock().unwrap().error(args);
    }

    fn concurrent(&self) -> Self::Concurrent {
        self.dup()
    }
}

/// This implementation just calls [`flush`](ConcurrentWrapper::flush),
///     type Concurrent = Option<P::Concurrent>;
///
/// to guarantee that all updates are correctly passed to the underlying logger.
impl<P: ProgressLog> Drop for ConcurrentWrapper<P> {
    fn drop(&mut self) {
        self.flush();
    }
}

impl Display for ConcurrentWrapper {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.inner.lock().unwrap().fmt(f)
    }
}

/// Convenience macro specifying that no (concurrent) logging should be
/// performed.
#[macro_export]
macro_rules! no_logging {
    () => {
        &mut Option::<dsi_progress_logger::ConcurrentWrapper::<dsi_progress_logger::ProgressLogger>>::None
    };
}

pub mod prelude {
    pub use super::{
        concurrent_progress_logger, no_logging, progress_logger, ConcurrentProgressLog,
        ConcurrentWrapper, ProgressLog, ProgressLogger,
    };
}
