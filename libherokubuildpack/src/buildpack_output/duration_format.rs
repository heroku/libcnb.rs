use std::time::Duration;

#[must_use]
pub(crate) fn human(duration: &Duration) -> String {
    let hours = (duration.as_secs() / 3600) % 60;
    let minutes = (duration.as_secs() / 60) % 60;
    let seconds = duration.as_secs() % 60;
    let milliseconds = duration.subsec_millis();

    if hours > 0 {
        format!("{hours}h {minutes}m {seconds}s")
    } else if minutes > 0 {
        format!("{minutes}m {seconds}s")
    } else if seconds > 0 || milliseconds > 100 {
        format!("{seconds}.{milliseconds:0>3}s")
    } else {
        String::from("< 0.1s")
    }
}

#[cfg(test)]
mod test {
    use super::*;

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
