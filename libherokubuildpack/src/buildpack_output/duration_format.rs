use std::time::Duration;

// Returns the part of a duration only in milliseconds
pub(crate) fn milliseconds(duration: &Duration) -> u32 {
    duration.subsec_millis()
}

pub(crate) fn seconds(duration: &Duration) -> u64 {
    duration.as_secs() % 60
}

pub(crate) fn minutes(duration: &Duration) -> u64 {
    (duration.as_secs() / 60) % 60
}

pub(crate) fn hours(duration: &Duration) -> u64 {
    (duration.as_secs() / 3600) % 60
}

#[must_use]
pub(crate) fn human(duration: &Duration) -> String {
    let hours = hours(duration);
    let minutes = minutes(duration);
    let seconds = seconds(duration);
    let milliseconds = milliseconds(duration);

    if hours > 0 {
        format!("{hours}h {minutes}m {seconds}s")
    } else if minutes > 0 {
        format!("{minutes}m {seconds}s")
    } else if seconds > 0 || milliseconds > 100 {
        // 0.1
        format!("{seconds}.{milliseconds:0>3}s")
    } else {
        String::from("< 0.1s")
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_millis_and_seconds() {
        let duration = Duration::from_millis(1024);
        assert_eq!(24, milliseconds(&duration));
        assert_eq!(1, seconds(&duration));
    }

    #[test]
    fn test_display_duration() {
        let duration = Duration::from_millis(99);
        assert_eq!("< 0.1s", human(&duration).as_str());

        let duration = Duration::from_millis(1024);
        assert_eq!("1.024s", human(&duration).as_str());

        let duration = Duration::from_millis(60 * 1024);
        assert_eq!("1m 1s", human(&duration).as_str());

        let duration = Duration::from_millis(3600 * 1024);
        assert_eq!("1h 1m 26s", human(&duration).as_str());
    }
}
