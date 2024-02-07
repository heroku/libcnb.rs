use std::time::Duration;

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
        let duration = Duration::ZERO;
        assert_eq!(human(&duration), "< 0.1s");

        let duration = Duration::from_millis(99);
        assert_eq!(human(&duration), "< 0.1s");

        let duration = Duration::from_millis(100);
        assert_eq!(human(&duration), "0.100s");

        let duration = Duration::from_millis(999);
        assert_eq!(human(&duration), "0.999s");

        let duration = Duration::from_millis(1000);
        assert_eq!(human(&duration), "1.000s");

        let duration = Duration::from_millis(60 * 1000 - 1);
        assert_eq!(human(&duration), "59.999s");

        let duration = Duration::from_millis(60 * 1000);
        assert_eq!(human(&duration), "1m 0s");

        let duration = Duration::from_millis(60 * 60 * 1000 - 1);
        assert_eq!(human(&duration), "59m 59s");

        let duration = Duration::from_millis(60 * 60 * 1000);
        assert_eq!(human(&duration), "1h 0m 0s");

        let duration = Duration::from_millis(75 * 60 * 1000 - 1);
        assert_eq!(human(&duration), "1h 14m 59s");
    }
}
