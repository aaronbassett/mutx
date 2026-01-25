use crate::error::{MutxError, Result};
use std::time::Duration;

/// Parse a duration string like "30s", "5m", "2h", "7d"
/// Defaults to seconds if no unit specified
pub fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim();

    if s.is_empty() {
        return Err(MutxError::InvalidDuration {
            input: s.to_string(),
            message: "empty string".to_string(),
        });
    }

    let (num_str, unit) = if let Some(stripped) = s.strip_suffix('s') {
        (stripped, 's')
    } else if let Some(stripped) = s.strip_suffix('m') {
        (stripped, 'm')
    } else if let Some(stripped) = s.strip_suffix('h') {
        (stripped, 'h')
    } else if let Some(stripped) = s.strip_suffix('d') {
        (stripped, 'd')
    } else {
        // No unit, assume seconds
        (s, 's')
    };

    let value: u64 = num_str.parse().map_err(|_| MutxError::InvalidDuration {
        input: s.to_string(),
        message: "expected format: NUMBER[s|m|h|d] (e.g., '30s', '5m', '2h', '7d')"
            .to_string(),
    })?;

    let seconds = match unit {
        's' => value,
        'm' => value * 60,
        'h' => value * 60 * 60,
        'd' => value * 60 * 60 * 24,
        _ => unreachable!(),
    };

    Ok(Duration::from_secs(seconds))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_units() {
        assert_eq!(parse_duration("1s").unwrap().as_secs(), 1);
        assert_eq!(parse_duration("1m").unwrap().as_secs(), 60);
        assert_eq!(parse_duration("1h").unwrap().as_secs(), 3600);
        assert_eq!(parse_duration("1d").unwrap().as_secs(), 86400);
    }
}
