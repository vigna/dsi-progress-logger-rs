/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

#![doc = include_str!("../README.md")]

use log::info;
use num_format::{Locale, ToFormattedString};
use pluralizer::pluralize;
use std::fmt::{Arguments, Display, Formatter, Result};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use sysinfo::{Pid, ProcessExt, ProcessRefreshKind, RefreshKind, System, SystemExt};
mod utils;
pub use utils::*;

/**

Logging trait.

Implemented by [`ProgressLog`] and by `Option<ProgressLog>`. This approach makes it possible to
pass as a [`ProgressLog`] either a [`ProgressLogger`], an `Option<ProgressLogger>`, or even
`Option::<ProgressLogger>::None`.

*/
pub trait ProgressLog {
    /// Forces a log of `self` assuming `now` is the current time.
    ///
    /// This is a low-level method that should not be called directly.
    fn log(&mut self, now: Instant);

    /// Logs `self` if it is time to log.
    ///
    /// This is a low-level method that should not be called directly.
    fn log_if(&mut self);

    /// Sets the display of memory information.
    ///
    /// Memory information include:
    /// - the [resident-set size](sysinfo::Process::memory) of the process that created the logger;
    /// - the [virtual-memory size](sysinfo::Process::virtual_memory) of the process that created the logger;
    /// - the [available memory](sysinfo::System::available_memory);
    /// - the [free memory](`sysinfo::System::free_memory);
    /// - the [total amount](sysinfo::System::total_memory) of memory.
    fn display_memory(&mut self, display_memory: bool) -> &mut Self;

    /// Sets the name of an item.
    fn item_name(&mut self, item_name: impl AsRef<str>) -> &mut Self;

    /// Sets the log interval.
    fn log_interval(&mut self, log_interval: Duration) -> &mut Self;

    /// Sets the expected number of updates.
    ///
    /// If not [`None`],
    /// the logger will display the percentage of completion and
    /// an estimate of the time to completion.
    fn expected_updates(&mut self, expected_updates: Option<usize>) -> &mut Self;

    /// Sets the time unit to use for speed.
    ///
    /// If not [`None`], the logger will always display the speed in this unit
    /// instead of making a choice of readable unit based on the elapsed time. Moreover, large numbers
    /// will not be thousands separated. This behavior is useful when the output of the logger must be parsed.
    fn time_unit(&mut self, time_unit: Option<TimeUnit>) -> &mut Self;

    /// Set whether to display additionally the speed achieved during the last log interval.
    fn local_speed(&mut self, local_speed: bool) -> &mut Self;

    /// Sets the [`log`] target.
    ///
    /// This should often be the path of the module logging progress,
    /// which is obtained with [`std::module_path!`].
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

    /// Starts the logger, displaying the given message.
    ///
    /// You can pass the empty string to display nothing.
    fn start(&mut self, msg: impl AsRef<str>);

    /// Increases the count and check whether it is time to log.
    fn update(&mut self);

    /// Sets the count and check whether it is time to log.
    fn update_with_count(&mut self, count: usize);

    /// Increases the count but checks whether it is time to log only after an
    /// implementation-defined number of calls.
    ///
    /// Useful for very short activities with respect to which  checking the
    /// time is expensive.
    fn light_update(&mut self);

    /// Increases the count and forces a log.
    fn update_and_display(&mut self);

    /// Stops the logger, fixing the final time.
    fn stop(&mut self);

    /// Stops the logger, print `Completed.`, and display the final stats. The
    /// number of expected updates will be cleared.
    fn done(&mut self);

    /// Stops the logger, sets the count, prints `Completed.`, and displays the
    /// final stats. The number of expected updates will be cleared.
    ///
    /// This method is particularly useful in two circumstances:
    /// * you have updated the logger with some approximate values (e.g., in a
    ///   multicore computation) but before printing the final stats you want
    ///   the internal counter to contain an exact value;
    /// * you have used the logger as a handy timer, calling just
    ///   [`start`](#fields.start) and this method.
    fn done_with_count(&mut self, count: usize);

    /// Returns the elapsed time since the logger was started, or `None` if the
    /// logger has not been started.
    fn elapsed(&self) -> Option<Duration>;

    /// Refreshes memory information, if previously requested with
    /// [`display_memory`](#method.display_memory). You do not need to call this
    /// method unless you display the logger manually.
    fn refresh(&mut self);

    /// Outputs the given message.
    ///
    /// For maximum flexibility, this method takes as argument the result of a
    /// [`std::format_args!`] macro. Note that there will be no output if the
    /// logger is the [`None`] variant.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use dsi_progress_logger::*;
    ///
    ///    env_logger::builder()
    ///        .filter_level(log::LevelFilter::Info)
    ///        .try_init()?;
    ///
    /// let logger_name = "my_logger";
    /// let mut pl = progress_logger!();
    /// pl.info(format_args!("My logger named {}", logger_name));
    /// #     Ok(())
    /// # }
    /// ```
    fn info(&self, args: Arguments<'_>);

    /// Clones the logger, returning a logger with the same setup but with all
    /// the counters reset.
    ///
    /// Note that we cannot simply implement the [`Clone`] trait because we will
    /// need this method also for the [`Option`] variant.
    fn clone(&self) -> Self;
}

impl<P: ProgressLog> ProgressLog for Option<P> {
    fn log(&mut self, now: Instant) {
        if let Some(pl) = self {
            pl.log(now);
        }
    }

    fn log_if(&mut self) {
        if let Some(pl) = self {
            pl.log_if();
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

    fn update_with_count(&mut self, count: usize) {
        if let Some(pl) = self {
            pl.update_with_count(count);
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

    fn refresh(&mut self) {
        if let Some(pl) = self {
            pl.refresh();
        }
    }

    fn info(&self, args: Arguments<'_>) {
        if let Some(pl) = self {
            pl.info(args);
        }
    }

    fn clone(&self) -> Self {
        self.as_ref().map(|pl| pl.clone())
    }
}

/**

An implementation of [`ProgressLog`] with output generated using
the [`log`](https://docs.rs/log) crate at the `info` level.

*/
pub struct ProgressLogger {
    /// The name of an item. Defaults to `item`.
    item_name: String,
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

/// Macro to create a [`ProgressLogger`] with default log target set to
/// [`std::module_path!`], and key-value pairs instead of setters.
///
/// # Examples
///
///
/// ```rust
/// use dsi_progress_logger::prelude::*;
///
/// let mut pl = progress_logger!(item_name="pumpkin", display_memory=true);
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

impl Default for ProgressLogger {
    fn default() -> Self {
        Self {
            item_name: "item".into(),
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

impl ProgressLogger {
    /// Calls to [light_update](#method.light_update) will cause a call to
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
            pluralize(&self.item_name, 2, false),
            time_unit_speed.label(),
            seconds_per_item / time_unit_timing.as_seconds(),
            time_unit_timing.label(),
            self.item_name
        ))?;

        Ok(())
    }
}

impl ProgressLog for ProgressLogger {
    fn log(&mut self, now: Instant) {
        self.refresh();
        info!(target: &self.log_target, "{}", self);
        self.last_count = self.count;
        self.last_log_time = now;
        self.next_log_time = now + self.log_interval;
    }

    fn log_if(&mut self) {
        let now = Instant::now();
        if self.next_log_time <= now {
            self.log(now);
        }
    }

    fn display_memory(&mut self, display_memory: bool) -> &mut Self {
        match (display_memory, &self.system) {
            (true, None) => {
                self.system = Some(System::new_with_specifics(RefreshKind::new().with_memory()));
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
            system.refresh_process_specifics(self.pid, ProcessRefreshKind::new());
        }
    }

    fn update(&mut self) {
        self.count += 1;
        self.log_if();
    }

    fn update_with_count(&mut self, count: usize) {
        self.count += count;
        self.log_if();
    }

    /// Increases the count and, once every
    /// [`LIGHT_UPDATE_MASK`](#fields.LIGHT_UPDATE_MASK) + 1 calls, check
    /// whether it is time to log.
    #[inline(always)]
    fn light_update(&mut self) {
        self.count += 1;
        if (self.count & Self::LIGHT_UPDATE_MASK) == 0 {
            self.log_if();
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

    fn info(&self, args: Arguments<'_>) {
        info!(target: &self.log_target, "{}", std::fmt::format(args));
    }

    #[allow(clippy::manual_map)]
    fn clone(&self) -> Self {
        Self {
            item_name: self.item_name.clone(),
            log_interval: self.log_interval,
            time_unit: self.time_unit,
            local_speed: self.local_speed,
            system: match self.system {
                Some(_) => Some(System::new_with_specifics(RefreshKind::new().with_memory())),
                None => None,
            },
            ..ProgressLogger::default()
        }
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
                        pluralize(&self.item_name, self.count as isize, false)
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
                    pluralize(&self.item_name, self.count as isize, false),
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

/// A concurrent implementation of [`ProgressLog`] with output generated using
/// the [`log`](https://docs.rs/log) crate at the `info` level.
///
/// Instances can be cloned to give several threads their own logger. The
/// methods [`update`](ProgressLog::update) and
/// [`update_with_count`](ProgressLog::update_with_count) buffer the increment
/// and add it to the underlying logger only when the buffer reaches a
/// threshold; this prevents locking the underlying logger too often. The
/// threshold is set at creation using the methods
/// [`with_threshold`](Self::with_threshold) and
/// [`wrap_with_threshold`](Self::wrap_with_threshold), or by
/// calling the method [`threshold`](Self::threshold).
///
///
/// The method [`light_update`](ProgressLog::light_update), as in the case of
/// [`ProgressLogger`], further delays updates using an even faster check.

pub struct ConcurrentProgressLogger<P: ProgressLog = ProgressLogger> {
    /// An atomically reference counted, mutex-protected logger.
    inner: Arc<Mutex<P>>,
    /// The number of items processed by the current thread.
    local_count: u32,
    /// The threshold for updating the underlying logger.
    threshold: u32,
}

/// Macro to create a [`ConcurrentProgressLogger`] with default log target set to
/// [`std::module_path!`], and key-value pairs instead of setters.
///
/// # Examples
///
/// ```rust
/// use dsi_progress_logger::prelude::*;
///
/// let mut pl = concurrent_progress_logger!(item_name="pumpkin", display_memory=true);
/// ```

#[macro_export]
macro_rules! concurrent_progress_logger {
    ($($method:ident = $arg:expr),* $(,)?) => {
        {
            let mut cpl = ::dsi_progress_logger::ConcurrentProgressLogger::default();
            ::dsi_progress_logger::ProgressLog::log_target(&mut cpl, ::std::module_path!());
            $(
                ::dsi_progress_logger::ProgressLog::$method(&mut cpl, $arg);
            )*
            cpl
        }
    }
}

impl Default for ConcurrentProgressLogger {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(ProgressLogger::default())),
            local_count: 0,
            threshold: Self::DEFAULT_THRESHOLD,
        }
    }
}

impl ConcurrentProgressLogger {
    /// Create a new [`ConcurrentProgressLogger`] wrapping a [`ProgressLogger`],
    /// using the [default threshold](Self::DEFAULT_THRESHOLD).
    pub fn new() -> Self {
        Self::with_threshold(Self::DEFAULT_THRESHOLD)
    }

    /// Create a new [`ConcurrentProgressLogger`] wrapping a [`ProgressLogger`],
    /// using the given threshold.
    pub fn with_threshold(threshold: u32) -> Self {
        Self {
            inner: Arc::new(Mutex::new(ProgressLogger::default())),
            local_count: 0,
            threshold,
        }
    }
}

impl<P: ProgressLog> ConcurrentProgressLogger<P> {
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
    /// Note concurrent loggers with the same underlying logger
    /// have independent thresholds.
    pub fn threshold(&mut self, threshold: u32) -> &mut Self {
        self.threshold = threshold;
        self
    }

    /// Wrap a given [`ProgressLog`] in a [`ConcurrentProgressLogger`]
    /// using the [default threshold](Self::DEFAULT_THRESHOLD).
    pub fn wrap(inner: P) -> Self {
        Self {
            inner: Arc::new(Mutex::new(inner)),
            local_count: 0,
            threshold: Self::DEFAULT_THRESHOLD,
        }
    }

    /// Wrap a given [`ProgressLog`] in a [`ConcurrentProgressLogger`] using a
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

impl<P: ProgressLog> ProgressLog for ConcurrentProgressLogger<P> {
    fn log(&mut self, now: Instant) {
        self.inner.lock().unwrap().log(now);
        self.local_count = 0;
    }

    fn log_if(&mut self) {
        self.inner.lock().unwrap().log_if();
        self.local_count = 0;
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
    fn update_with_count(&mut self, count: usize) {
        match (self.local_count as usize).checked_add(count) {
            None => {
                // Sum overflows, update in two steps
                let mut pl = self.inner.lock().unwrap();
                pl.update_with_count(self.local_count as _);
                pl.update_with_count(count);
                self.local_count = 0;
            }
            Some(total_count) => {
                if total_count >= self.threshold as usize {
                    // Threshold reached, time to flush to the inner ProgressLogConfig
                    self.inner.lock().unwrap().update_with_count(total_count);
                    self.local_count = 0;
                } else {
                    // total_count is lower than self.threshold, which is a u16;
                    // so total_count fits in u16.
                    self.local_count = total_count as u32;
                }
            }
        }
    }

    #[inline]
    fn light_update(&mut self) {
        self.local_count += 1;
        if (self.local_count & Self::LIGHT_UPDATE_MASK) == 0 {
            self.inner
                .lock()
                .unwrap()
                .update_with_count(self.local_count as _);
            self.local_count = 0;
        }
    }

    fn update_and_display(&mut self) {
        self.local_count += 1;
        self.inner
            .lock()
            .unwrap()
            .update_with_count(self.local_count as _);
        self.local_count = 0;
    }

    fn stop(&mut self) {
        self.inner.lock().unwrap().stop();
        self.local_count = 0;
    }

    fn done(&mut self) {
        self.inner.lock().unwrap().done();
        self.local_count = 0;
    }

    fn done_with_count(&mut self, count: usize) {
        self.inner.lock().unwrap().done_with_count(count);
        self.local_count = 0;
    }

    fn elapsed(&self) -> Option<Duration> {
        self.inner.lock().unwrap().elapsed()
    }

    fn refresh(&mut self) {
        self.inner.lock().unwrap().refresh();
    }

    fn info(&self, args: Arguments<'_>) {
        self.inner.lock().unwrap().info(args);
    }

    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            local_count: 0,
            threshold: self.threshold,
        }
    }
}

/// Flush count buffer to the inner [`ProgressLog`].
impl<P: ProgressLog> Drop for ConcurrentProgressLogger<P> {
    fn drop(&mut self) {
        self.flush();
    }
}
impl Display for ConcurrentProgressLogger {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.inner.lock().unwrap().fmt(f)
    }
}

#[macro_export]
macro_rules! no_logging {
    () => {
        &mut Option::<dsi_progress_logger::ProgressLogger>::None
    };
}

pub mod prelude {
    pub use super::{
        concurrent_progress_logger, no_logging, progress_logger, ConcurrentProgressLogger,
        ProgressLog, ProgressLogger,
    };
}
