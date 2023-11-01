/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

/*!
A tunable progress logger to log progress information about long-running activities.
It is a port of the Java class [`it.unimi.dsi.util.ProgressLogger`](https://dsiutils.di.unimi.it/docs/it/unimi/dsi/logging/ProgressLogger.html)
from the [DSI Utilities](https://dsiutils.di.unimi.it/).
Logging is based on the standard [`log`](https://docs.rs/log) crate at the `info` level.

To log the progress of an activity, you call [`start`](ProgressLogger::start). Then, each time you want to mark progress,
you call [`update`](ProgressLogger::update), which increases the item counter, and will log progress information
if enough time has passed since the last log. The time check happens only on multiples of
[`LIGHT_UPDATE_MASK`](ProgressLogger::LIGHT_UPDATE_MASK) + 1 in the case of [`light_update`](ProgressLogger::light_update),
which should be used when the activity has an extremely low cost that is comparable to that
of the time check (a call to [`Instant::now()`]) itself.

Some fields can be set at any time to customize the logger: please see the [documentation of the fields](ProgressLogger).
It is also possible to log used and free memory at each log interval by calling
[`display_memory`](ProgressLogger::display_memory). Memory is read from system data by the [`sysinfo`] crate, and
will be updated at each log interval (note that this will slightly slow down the logging process). Moreover,
since it is impossible to update the memory information from the [`Display::fmt`] implementation,
you should call [`refresh`](ProgressLogger::refresh) before displaying the logger
on your own.

At any time, displaying the progress logger will give you time information up to the present.
When the activity is over, you call [`stop`](ProgressLogger::stop), which fixes the final time, and
possibly display again the logger. [`done`](ProgressLogger::done) will stop the logger, print `Completed.`,
and display the final stats. There are also a few other utility methods that make it possible to
customize the logging process.

After you finished a run of the progress logger, can call [`start`](ProgressLogger::start)
again to measure another activity.

A typical call sequence to a progress logger is as follows:
```
use dsi_progress_logger::*;

stderrlog::new().init().unwrap();
let mut pl = ProgressLogger::default();
pl.item_name("pumpkin");
pl.start("Smashing pumpkins...");
for _ in 0..100 {
   // do something on each pumlkin
   pl.update();
}
pl.done();
```
A progress logger can also be used as a handy timer:
```
use dsi_progress_logger::*;

stderrlog::new().init().unwrap();
let mut pl = ProgressLogger::default();
pl.item_name("pumpkin");
pl.start("Smashing pumpkins...");
for _ in 0..100 {
   // do something on each pumlkin
}
pl.done_with_count(100);
```
This progress logger will display information about  memory usage:
```
use dsi_progress_logger::*;

stderrlog::new().init().unwrap();
let mut pl = ProgressLogger::default();
pl.display_memory(true);
```
*/
use log::info;
use num_format::{Locale, ToFormattedString};
use pluralizer::pluralize;
use std::fmt::{Display, Formatter, Result};
use std::time::{Duration, Instant};
use sysinfo::{Pid, ProcessExt, RefreshKind, System, SystemExt};

mod utils;
use utils::*;

/**!

Logging trait.

Implemented by [`ProgressLog`] and by `Option<ProgressLog>`. This approach makes it possible to
pass as a [`ProgressLog`] either a [`ProgressLogger`], an `Option<ProgressLogger>`, or even
`Option::<ProgressLogger>::None`.

*/
pub trait ProgressLog {
    /// Display memory information.
    fn display_memory(&mut self, display_memory: bool) -> &mut Self;

    /// Set the name of an item.
    fn item_name(&mut self, item_name: impl AsRef<str>) -> &mut Self;

    /// Set the log interval.
    fn log_interval(&mut self, log_interval: Duration) -> &mut Self;

    /// Set the expected number of updates.
    ///
    /// If not [`None`],
    /// the logger will display the percentage of completion and
    /// an estimate of the time to completion.
    fn expected_updates(&mut self, expected_updates: Option<usize>) -> &mut Self;

    /// Set the time unit to use for speed.
    ///
    /// If not [`None], the logger will always display the speed in this unit
    /// instead of making a choice of readable unit based on the elapsed time. Moreover, large numbers
    /// will not be thousands separated. This is useful when the output of the logger must be parsed.
    fn time_unit(&mut self, time_unit: Option<TimeUnit>) -> &mut Self;

    /// Set whether to display additionally the speed achieved during the last log interval.
    fn local_speed(&mut self, local_speed: bool) -> &mut Self;

    /// Start the logger, displaying the given message.
    ///
    /// You can pass the empty string to display nothing.
    fn start(&mut self, msg: impl AsRef<str>);

    /// Increase the count and check whether it is time to log.
    fn update(&mut self);

    /// Set the count and check whether it is time to log.
    fn update_with_count(&mut self, count: usize);

    /// Increase the count but check whether it is time log only after an
    /// implementation-defined number of calls.
    ///
    /// Useful for very short activities with respect to which  checking the time is expensive.
    fn light_update(&mut self);

    /// Increase the count and force a log.
    fn update_and_display(&mut self);

    /// Stop the logger, fixing the final time.
    fn stop(&mut self);

    /// Stop the logger, print `Completed.`, and display the final stats.
    /// The number of expected updates will be cleared.
    fn done(&mut self);

    /// Stop the logger, set the count, print `Completed.`, and display the final stats.
    /// The number of expected updates will be cleared.
    ///
    /// This method is particularly useful in two circumstances:
    /// * you have updated the logger with some approximate values (e.g., in a multicore computation) but before
    ///   printing the final stats you want the internal counter to contain an exact value;
    /// * you have used the logger as a handy timer, calling just [`start`](#fields.start) and this method.
    fn done_with_count(&mut self, count: usize);

    /// Return the elapsed time since the logger was started, or `None` if the logger has not been started.
    fn elapsed(&self) -> Option<Duration>;

    /// Refresh memory information, if previously requested with [`display_memory`](#method.display_memory).
    /// You do not need to call this method unless you display the logger manually.
    fn refresh(&mut self);

    /// Clone the logger, returning a logger with the same setup but with all the counters reset.
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

    /// Set whether to display additionally the speed achieved during the last log interval.
    fn local_speed(&mut self, local_speed: bool) -> &mut Self {
        if let Some(pl) = self {
            pl.local_speed(local_speed);
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

    fn clone(&self) -> Self {
        self.as_ref().map(|pl| pl.clone())
    }
}

/**

An implementation of [`ProgressLog`] with output generated using the [`log`](https://docs.rs/log) crate
at the `info` level.

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
    fn default() -> Self {
        Self {
            item_name: "item".into(),
            log_interval: Duration::from_secs(10),
            expected_updates: None,
            time_unit: None,
            local_speed: false,
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
    /// [`Instant::now`] only if the current count
    /// is a multiple of this mask plus one.
    pub const LIGHT_UPDATE_MASK: usize = (1 << 20) - 1;

    fn log(&mut self, now: Instant) {
        self.refresh();
        info!("{}", self);
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
    /// Chainable setter enabling memory display.
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
        if self.system.is_none() {
            self.system = Some(System::new_with_specifics(RefreshKind::new().with_memory()));
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

    fn start(&mut self, msg: impl AsRef<str>) {
        let now = Instant::now();
        self.start_time = Some(now);
        self.stop_time = None;
        self.count = 0;
        self.last_count = 0;
        self.last_log_time = now;
        self.next_log_time = now + self.log_interval;
        if !msg.as_ref().is_empty() {
            info!("{}", msg.as_ref());
        }
    }

    fn refresh(&mut self) {
        if let Some(system) = &mut self.system {
            system.refresh_memory();
            system.refresh_process(self.pid);
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

    /// Increase the count and, once every [`LIGHT_UPDATE_MASK`](#fields.LIGHT_UPDATE_MASK) + 1 calls, check whether it is time to log.
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
        info!("Completed.");
        // just to avoid wrong reuses
        self.expected_updates = None;
        info!("{}", self);
    }

    fn done_with_count(&mut self, count: usize) {
        self.count = count;
        self.done();
    }

    fn elapsed(&self) -> Option<Duration> {
        self.start_time?.elapsed().into()
    }

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

            if let Some(system) = &self.system {
                f.write_fmt(format_args!(
                    "; used/avail/free/total mem {}/{}B/{}B/{}B",
                    system
                        .process(self.pid)
                        .map(|process| humanize(process.memory() as _) + "B")
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
