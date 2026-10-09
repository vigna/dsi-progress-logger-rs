/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

/// A unit of time, from nanoseconds to days.
///
/// Time units are used to display timings and speeds. Usually the most
/// convenient unit is chosen automatically, but you can fix a specific unit
/// with [`ProgressLog::time_unit`](crate::ProgressLog::time_unit).
#[derive(Debug, Copy, Clone)]
pub enum TimeUnit {
    /// A nanosecond (10⁻⁹ seconds).
    NanoSeconds,
    /// A microsecond (10⁻⁶ seconds).
    MicroSeconds,
    /// A millisecond (10⁻³ seconds).
    MilliSeconds,
    /// A second.
    Seconds,
    /// A minute (60 seconds).
    Minutes,
    /// An hour (3600 seconds).
    Hours,
    /// A day (86400 seconds).
    Days,
}

impl TimeUnit {
    /// All time units, in increasing order of duration.
    pub const VALUES: [TimeUnit; 7] = [
        TimeUnit::NanoSeconds,
        TimeUnit::MicroSeconds,
        TimeUnit::MilliSeconds,
        TimeUnit::Seconds,
        TimeUnit::Minutes,
        TimeUnit::Hours,
        TimeUnit::Days,
    ];

    /// Returns the label used to display this time unit (e.g., `s` for
    /// [`Seconds`](TimeUnit::Seconds)).
    pub const fn label(&self) -> &'static str {
        match self {
            TimeUnit::NanoSeconds => "ns",
            TimeUnit::MicroSeconds => "μs",
            TimeUnit::MilliSeconds => "ms",
            TimeUnit::Seconds => "s",
            TimeUnit::Minutes => "m",
            TimeUnit::Hours => "h",
            TimeUnit::Days => "d",
        }
    }

    /// Returns the length of this time unit in seconds.
    pub const fn as_seconds(&self) -> f64 {
        match self {
            TimeUnit::NanoSeconds => 1.0e-9,
            TimeUnit::MicroSeconds => 1.0e-6,
            TimeUnit::MilliSeconds => 1.0e-3,
            TimeUnit::Seconds => 1.0,
            TimeUnit::Minutes => 60.0,
            TimeUnit::Hours => 3600.0,
            TimeUnit::Days => 86400.0,
        }
    }

    /// Returns the largest time unit not exceeding the given duration in
    /// seconds; it is used to display per-item timings.
    ///
    /// For durations shorter than a nanosecond,
    /// [`NanoSeconds`](TimeUnit::NanoSeconds) is returned.
    pub const fn nice_time_unit(seconds: f64) -> Self {
        let mut i = TimeUnit::VALUES.len();
        while i > 0 {
            i -= 1;
            if seconds >= TimeUnit::VALUES[i].as_seconds() {
                return TimeUnit::VALUES[i];
            }
        }
        TimeUnit::NanoSeconds
    }

    /// Returns the smallest time unit, starting from
    /// [`Seconds`](TimeUnit::Seconds), in which an activity taking the given
    /// number of seconds per item progresses at a speed of at least one item
    /// per unit; it is used to display speeds.
    ///
    /// For activities slower than one item per day, [`Days`](TimeUnit::Days)
    /// is returned.
    pub const fn nice_speed_unit(seconds: f64) -> Self {
        let mut i = 3;
        while i < TimeUnit::VALUES.len() {
            if seconds <= TimeUnit::VALUES[i].as_seconds() {
                return TimeUnit::VALUES[i];
            }
            i += 1;
        }
        TimeUnit::Days
    }

    /// Pretty-prints a duration expressed in milliseconds.
    ///
    /// Durations of less than a second are displayed as `{}ms`; longer
    /// durations are displayed as days, hours, minutes, and seconds (e.g.,
    /// `1d 3h 10m 5s`), discarding the sub-second part.
    pub fn pretty_print(milliseconds: u128) -> String {
        let mut result = String::new();

        if milliseconds < 1000 {
            return format!("{}ms", milliseconds);
        }

        let mut seconds = milliseconds / 1000;

        for unit in [TimeUnit::Days, TimeUnit::Hours, TimeUnit::Minutes] {
            let to_seconds = unit.as_seconds() as u128;
            if seconds >= to_seconds {
                result.push_str(&format!("{}{} ", seconds / to_seconds, unit.label(),));
                seconds %= to_seconds;
            }
        }

        result.push_str(&format!("{}s", seconds));

        result
    }
}

/// A format for speeds and per-item timings.
///
/// The format is set with
/// [`ProgressLog::speed_format`](crate::ProgressLog::speed_format).
#[derive(Debug, Copy, Clone)]
pub enum SpeedFormat {
    /// Two decimal digits (e.g., `346588344.72`).
    Plain,
    /// Two decimal digits and commas as thousands separators (e.g.,
    /// `346,588,344.72`).
    Separated,
    /// Two decimal digits and an SI prefix, as in [`humanize`] (e.g.,
    /// `346.59M`).
    Si,
}

impl SpeedFormat {
    /// Formats a value in this format.
    pub(crate) fn format(self, val: f64) -> String {
        match self {
            SpeedFormat::Plain => format!("{val:.2}"),
            SpeedFormat::Separated => separate_thousands(val),
            SpeedFormat::Si => humanize(val),
        }
    }
}

/// Scales down a value by powers of 1000, returning the scaled value and the
/// associated SI prefix.
///
/// For example, `scale(1234.0)` returns `(1.234, "k")`. Values of a
/// septillion (10²⁴) or more are all scaled to yottas (`Y`).
pub const fn scale(mut val: f64) -> (f64, &'static str) {
    const UNITS: &[&str] = &["", "k", "M", "G", "T", "P", "E", "Z", "Y"];
    let mut i = 0;
    while i < UNITS.len() - 1 {
        if val < 1000.0 {
            return (val, UNITS[i]);
        }
        val /= 1000.0;
        i += 1;
    }

    (val, "Y")
}

/// Formats a value scaled with [`scale`] using two decimal digits (e.g.,
/// `1.23G`).
pub fn humanize(val: f64) -> String {
    let (val, unit) = scale(val);
    format!("{:.2}{}", val, unit)
}

/// Formats a value using two decimal digits and commas as thousands separators
/// (e.g., `1,234,567.89`).
///
/// The digits are exactly those of `format!("{:.2}", val)`; non-finite values
/// are formatted in the same way.
fn separate_thousands(val: f64) -> String {
    let s = format!("{val:.2}");
    // Non-finite values (inf, NaN) have no decimal point
    let Some(point) = s.find('.') else {
        return s;
    };
    let (int, frac) = s.split_at(point);
    let (sign, int) = int.split_at(usize::from(int.starts_with('-')));
    let mut result = String::with_capacity(s.len() + int.len() / 3);
    result.push_str(sign);
    for (i, digit) in int.chars().enumerate() {
        if i != 0 && (int.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(digit);
    }
    result.push_str(frac);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale() {
        assert_eq!(scale(1000.0), (1.0, "k"));
        assert_eq!(scale(300_000.0), (300.0, "k"));
        assert_eq!(scale(1_000_000_000.0), (1.0, "G"));
    }

    #[test]
    fn test_humanize() {
        assert_eq!(humanize(1000.0), "1.00k");
        assert_eq!(humanize(12_345.0), "12.35k");
        assert_eq!(humanize(1_234_567_890.0), "1.23G");
        assert_eq!(humanize(1e24), "1.00Y");
        // Values of 10²⁷ or more stay at the last prefix
        assert_eq!(humanize(1e27), "1000.00Y");
        assert_eq!(humanize(1.5e30), "1500000.00Y");
    }

    #[test]
    fn test_separate_thousands() {
        assert_eq!(separate_thousands(0.0), "0.00");
        assert_eq!(separate_thousands(999.994), "999.99");
        // Rounding can add a digit
        assert_eq!(separate_thousands(999.999), "1,000.00");
        assert_eq!(separate_thousands(123_456.0), "123,456.00");
        assert_eq!(separate_thousands(1_234_567.891), "1,234,567.89");
        assert_eq!(separate_thousands(346_588_344.72), "346,588,344.72");
        assert_eq!(separate_thousands(-123_456.0), "-123,456.00");
        assert_eq!(separate_thousands(f64::INFINITY), "inf");
        assert_eq!(separate_thousands(f64::NAN), "NaN");
    }

    #[test]
    fn test_pretty_print() {
        assert_eq!(TimeUnit::pretty_print(500), "500ms");
        assert_eq!(TimeUnit::pretty_print(1000), "1s");
        assert_eq!(TimeUnit::pretty_print(1500), "1s");
        assert_eq!(TimeUnit::pretty_print(90_061_000), "1d 1h 1m 1s");
    }

    #[test]
    fn test_nice_units() {
        assert_eq!(TimeUnit::nice_time_unit(0.5).label(), "ms");
        assert_eq!(TimeUnit::nice_time_unit(100.0).label(), "m");
        assert_eq!(TimeUnit::nice_speed_unit(0.5).label(), "s");
        assert_eq!(TimeUnit::nice_speed_unit(100.0).label(), "h");
    }
}
