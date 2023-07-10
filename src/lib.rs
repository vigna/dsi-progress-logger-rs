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
use dsi_progress_logger::ProgressLogger;

stderrlog::new().init().unwrap();
let mut pl = ProgressLogger::default();
pl.item_name = "pumpkin";
pl.start("Smashing pumpkins...");
for _ in 0..100 {
   // do something on each pumlkin
   pl.update();
}
pl.done();
```
A progress logger can also be used as a handy timer:
```
use dsi_progress_logger::ProgressLogger;

stderrlog::new().init().unwrap();
let mut pl = ProgressLogger::default();
pl.item_name = "pumpkin";
pl.start("Smashing pumpkins...");
for _ in 0..100 {
   // do something on each pumlkin
}
pl.done_with_count(100);
```
This progress logger will display information about  memory usage:
```
use dsi_progress_logger::ProgressLogger;

stderrlog::new().init().unwrap();
let mut pl = ProgressLogger::default().display_memory();
```
*/
use log::info;
use num_format::{Locale, ToFormattedString};
use pluralizer::pluralize;
use std::fmt::{Display, Formatter, Result};
use std::sync::atomic::{compiler_fence, fence, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use sysinfo::{Pid, ProcessExt, RefreshKind, System, SystemExt};

mod utils;
use utils::*;

#[repr(align(64))]
/// A counter that uses a whole cache line so that it can be updated
/// without causing false sharing.
struct ConcurrentCounter(AtomicUsize);

/// A struct that holds both times because we alwasy need to update both
/// at the same time and this way we can use just one Mutex.
struct LogTime {
    last_log_time: Instant,
    next_log_time: Instant,
}

pub struct ProgressLogger<'a> {
    /// The name of an item. Defaults to `item`.
    pub item_name: &'a str,
    /// The log interval. Defaults to 10 seconds.
    pub log_interval: Duration,
    /// The expected number of updates. If set, the logger will display the percentage of completion and
    /// an estimate of the time to completion.
    pub expected_updates: Option<usize>,
    /// The time unit to use for speed. If set, the logger will always display the speed in this unit
    /// instead of making a choice of readable unit based on the elapsed time. Moreover, large numbers
    /// will not be thousands separated. This is useful when the output of the logger must be parsed.
    pub time_unit: Option<TimeUnit>,
    /// Display additionally the speed achieved during the last log interval.
    pub local_speed: bool,
    start_time: Option<Instant>,
    log_time: Mutex<LogTime>,
    stop_time: Option<Instant>,
    /// Thread specific counters
    concurrent_counts: Vec<ConcurrentCounter>,
    count: AtomicUsize,
    last_count: AtomicUsize,
    /// Display additionally the amount of used and free memory using this [`sysinfo::System`]
    system: Option<Mutex<System>>,
    /// The pid of the current process
    pid: Pid,
}

impl<'a> Default for ProgressLogger<'a> {
    fn default() -> Self {
        Self {
            item_name: "item",
            log_interval: Duration::from_secs(10),
            expected_updates: None,
            time_unit: None,
            local_speed: false,
            start_time: None,
            log_time: Mutex::new(LogTime {
                last_log_time: Instant::now(),
                next_log_time: Instant::now(),
            }),
            stop_time: None,
            concurrent_counts: (0..rayon::current_num_threads())
                .map(|_| ConcurrentCounter(AtomicUsize::new(0)))
                .collect(),
            count: AtomicUsize::new(0),
            last_count: AtomicUsize::new(0),
            system: None,
            pid: Pid::from(std::process::id() as usize),
        }
    }
}

impl<'a> ProgressLogger<'a> {
    /// The exponent of 2 used to compute [`LIGHT_UPDATE_MASK`]
    pub const LIGHT_UPDATE_EXP: usize = 20;
    /// Calls to [light_update](#method.light_update) will cause a call to
    /// [`Instant::now`] only if the current count
    /// is a multiple of this mask plus one.
    pub const LIGHT_UPDATE_MASK: usize = (1 << Self::LIGHT_UPDATE_EXP) - 1;

    /// Calls to [light_update_par](#method.light_update) will read the other
    /// threads counters and update the count, which might call [`Instant::now`]
    /// if the current count is a multiple of this mask plus one.
    pub const LIGHT_UPDATE_MASK_PAR: usize = (1 << 15) - 1;

    /// The atomico ordering used by all atomic operations.
    const ORDERING: Ordering = Ordering::Relaxed;

    /// Start the logger, displaying the given message.
    pub fn start<T: AsRef<str>>(&mut self, msg: T) {
        let now = Instant::now();
        self.start_time = Some(now);
        self.stop_time = None;
        self.count = AtomicUsize::new(0);
        self.last_count = AtomicUsize::new(0);
        {
            let mut log_time = self.log_time.lock().unwrap();
            log_time.last_log_time = now;
            log_time.next_log_time = now + self.log_interval;
        }
        info!("{}", msg.as_ref());
    }

    /// Chainable setter enabling memory display.
    pub fn display_memory(mut self) -> Self {
        if self.system.is_none() {
            self.system = Some(Mutex::new(System::new_with_specifics(
                RefreshKind::new().with_memory(),
            )));
        }
        self
    }

    /// Refresh memory information, if previously requested with [`display_memory`](#methods.display_memory).
    /// You do not need to call this method unless you display the logger manually.
    pub fn refresh(&self) {
        if let Some(system) = &self.system {
            let mut system = system.lock().unwrap();
            system.refresh_memory();
            system.refresh_process(self.pid);
        }
    }

    fn log(&self, count: usize, now: Instant) {
        self.refresh();
        info!("{}", self);
        self.last_count.store(count, Self::ORDERING);
        {
            let mut log_time = self.log_time.lock().unwrap();
            log_time.last_log_time = now;
            log_time.next_log_time = now + self.log_interval;
        }
    }

    fn log_if(&self, count: usize) {
        let now = Instant::now();
        let next_log_time = { self.log_time.lock().unwrap().next_log_time };
        if next_log_time <= now {
            self.log(count, now);
        }
    }

    /// Increase the count and check whether it is time to log.
    pub fn update(&mut self) {
        let count = self.count.fetch_add(1, Self::ORDERING);
        self.log_if(count);
    }

    /// Increase the count and check whether it is time to log.
    pub fn update_par(&self) {
        let count = self.count.fetch_add(1, Self::ORDERING);
        self.log_if(count);
    }

    /// Set the count and check whether it is time to log.
    pub fn update_with_count(&mut self, count: usize) {
        let count = self.count.fetch_add(count, Self::ORDERING);
        self.log_if(count);
    }

    /// Set the count and check whether it is time to log.
    pub fn update_with_count_par(&self, count: usize) {
        let count = self.count.fetch_add(count, Self::ORDERING);
        self.log_if(count);
    }

    /// Increase the count and, once every [`LIGHT_UPDATE_MASK`](#fields.LIGHT_UPDATE_MASK) + 1 calls, check whether it is time to log.
    #[inline(always)]
    pub fn light_update(&mut self) {
        let count = self.count.fetch_add(1, Self::ORDERING);
        if (count & Self::LIGHT_UPDATE_MASK) == 0 {
            self.log_if(count);
        }
    }

    /// Increase the count and, once every [`LIGHT_UPDATE_MASK`](#fields.LIGHT_UPDATE_MASK) + 1 calls, check whether it is time to log.
    #[inline(always)]
    pub fn light_update_par(&self) {
        // Update the counter for this thread
        let counter = self.concurrent_counts[rayon::current_thread_index().unwrap()]
            .0
            .fetch_add(1, Self::ORDERING);
        // if this counter is big enough, update the global counter
        // and check whether it is time to log
        if (counter & Self::LIGHT_UPDATE_MASK_PAR) == 0 {
            // since we are pushing multiple value, we can't just do & MASK == 0
            // but we want to know if with this update, we passed a multiple
            // of (mask + 1)
            let (prev_count, new_count) = loop {
                let prev_count = self.count.load(Self::ORDERING);
                let new_count = prev_count + counter;
                if self
                    .count
                    .compare_exchange_weak(prev_count, new_count, Self::ORDERING, Self::ORDERING)
                    .is_ok()
                {
                    break (prev_count, new_count);
                }
            };
            // if the higher bits changed, we passed a multiple of (mask + 1)
            if (prev_count >> Self::LIGHT_UPDATE_EXP) != (new_count >> Self::LIGHT_UPDATE_EXP) {
                self.log_if(new_count);
            }
        }
    }

    /// Increase the count and force a log.
    pub fn update_and_display(&mut self) {
        let count = self.count.fetch_add(1, Self::ORDERING);
        self.log(count, Instant::now());
    }

    /// Stop the logger, fixing the final time.
    pub fn stop(&mut self) {
        self.stop_time = Some(Instant::now());
        self.expected_updates = None;
    }

    /// Stop the logger, print `Completed.`, and display the final stats. The number of expected updates will be cleared.
    pub fn done(&mut self) {
        self.stop();
        info!("Completed.");
        // just to avoid wrong reuses
        self.expected_updates = None;
        info!("{}", self);
    }

    /// Stop the logger, set the count, print `Completed.`, and display the final stats.
    /// The number of expected updates will be cleared.
    ///
    /// This method is particularly useful in two circumstances:
    /// * you have updated the logger with some approximate values (e.g., in a multicore computation) but before
    ///   printing the final stats you want the internal counter to contain an exact value;
    /// * you have used the logger as a handy timer, calling just [`start`](#fields.start) and this method.

    pub fn done_with_count(&mut self, count: usize) {
        self.count.store(count, Self::ORDERING);
        fence(Ordering::SeqCst);
        self.done();
    }

    /// Return the elapsed time since the logger was started, or `None` if the logger has not been started.
    pub fn elapsed(&self) -> Option<Duration> {
        self.start_time?.elapsed().into()
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
            pluralize(self.item_name, 2, false),
            time_unit_speed.label(),
            seconds_per_item / time_unit_timing.as_seconds(),
            time_unit_timing.label(),
            self.item_name
        ))?;

        Ok(())
    }
}

impl<'a> Display for ProgressLogger<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        if let Some(start_time) = self.start_time {
            // retreive the data from the atomic variables trying to be as
            // close as possible to the actual values
            fence(Ordering::SeqCst);
            compiler_fence(Ordering::SeqCst);
            let last_log_time = { self.log_time.lock().unwrap().last_log_time };
            let last_count = self.last_count.load(Self::ORDERING);
            let (count, count_fmtd) = if self.time_unit.is_none() {
                let count = self.count.load(Self::ORDERING);
                (count, count.to_formatted_string(&Locale::en))
            } else {
                let count = self.count.load(Self::ORDERING);
                (count, count.to_string())
            };
            compiler_fence(Ordering::SeqCst);
            fence(Ordering::SeqCst);

            if let Some(stop_time) = self.stop_time {
                let elapsed = stop_time - start_time;
                let seconds_per_item = elapsed.as_secs_f64() / count as f64;

                f.write_fmt(format_args!(
                    "Elapsed: {}",
                    TimeUnit::pretty_print(elapsed.as_millis())
                ))?;

                if count != 0 {
                    f.write_fmt(format_args!(
                        " [{} {}, ",
                        count_fmtd,
                        pluralize(self.item_name, count as isize, false)
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
                    pluralize(self.item_name, count as isize, false),
                    TimeUnit::pretty_print(elapsed.as_millis()),
                ))?;

                let seconds_per_item = elapsed.as_secs_f64() / count as f64;
                self.fmt_timing_speed(f, seconds_per_item)?;

                if let Some(expected_updates) = self.expected_updates {
                    let millis_to_end: u128 = (expected_updates.saturating_sub(count) as u128
                        * elapsed.as_millis())
                        / (count as u128 + 1);
                    f.write_fmt(format_args!(
                        "; {:.2}% done, {} to end",
                        100.0 * count as f64 / expected_updates as f64,
                        TimeUnit::pretty_print(millis_to_end)
                    ))?;
                }

                if self.local_speed && self.stop_time.is_none() {
                    f.write_fmt(format_args!(" ["))?;

                    let elapsed = now - last_log_time;
                    let seconds_per_item = elapsed.as_secs_f64() / (count - last_count) as f64;
                    self.fmt_timing_speed(f, seconds_per_item)?;

                    f.write_fmt(format_args!("]"))?;
                }
            }

            if let Some(system) = &self.system {
                let system = system.lock().unwrap();
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
