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
    /// stderrlog::new().verbosity(2).init()?;
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

    /// Increases the count but checks whether it is time log only after an
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
    /// stderrlog::new().verbosity(2).init()?;
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
    fn clone(&self) -> Self;
}

impl<P: ProgressLog> ProgressLog for Option<P> {
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
            let mut pl = dsi_progress_logger::ProgressLogger::default();
            pl.log_target(std::module_path!());
            $(
                pl.$method($arg);
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

pub mod prelude {
    pub use super::{progress_logger, ProgressLog, ProgressLogger};
}
