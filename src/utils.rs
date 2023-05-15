#[derive(Debug, Copy, Clone)]

pub enum TimeUnit {
    NanoSeconds,
    MicroSeconds,
    MilliSeconds,
    Seconds,
    Minutes,
    Hours,
    Days,
}

impl TimeUnit {
    pub const VALUES: [TimeUnit; 7] = [
        TimeUnit::NanoSeconds,
        TimeUnit::MicroSeconds,
        TimeUnit::MilliSeconds,
        TimeUnit::Seconds,
        TimeUnit::Minutes,
        TimeUnit::Hours,
        TimeUnit::Days,
    ];

    pub fn label(&self) -> &'static str {
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

    pub fn as_seconds(&self) -> f64 {
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

    pub fn nice_time_unit(seconds: f64) -> Self {
        for unit in TimeUnit::VALUES.iter().rev() {
            if seconds >= unit.as_seconds() {
                return *unit;
            }
        }
        TimeUnit::NanoSeconds
    }

    pub fn nice_speed_unit(seconds: f64) -> Self {
        for unit in TimeUnit::VALUES[3..].iter() {
            if seconds <= unit.as_seconds() {
                return *unit;
            }
        }
        TimeUnit::Days
    }

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

pub fn scale(mut val: f64) -> (f64, &'static str) {
    const UNITS: &[&str] = &["", "k", "M", "G", "T", "P", "E", "Z", "Y"];
    for unit in UNITS.iter() {
        if val < 1000.0 {
            return (val, unit);
        }
        val /= 1000.0;
    }

    (val, "Y")
}

pub fn humanize(val: f64) -> String {
    let (val, unit) = scale(val);
    format!("{:.2}{}", val, unit)
}

#[cfg(test)]

mod test {
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
    }
}
