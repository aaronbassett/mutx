use mutx::utils::parse_duration;
use std::time::Duration;

#[test]
fn test_parse_seconds() {
    assert_eq!(parse_duration("30s").unwrap(), Duration::from_secs(30));
    assert_eq!(parse_duration("45").unwrap(), Duration::from_secs(45));
}

#[test]
fn test_parse_minutes() {
    assert_eq!(parse_duration("5m").unwrap(), Duration::from_secs(300));
}

#[test]
fn test_parse_hours() {
    assert_eq!(parse_duration("2h").unwrap(), Duration::from_secs(7200));
}

#[test]
fn test_parse_days() {
    assert_eq!(parse_duration("7d").unwrap(), Duration::from_secs(604800));
}

#[test]
fn test_parse_invalid_format() {
    assert!(parse_duration("invalid").is_err());
    assert!(parse_duration("10x").is_err());
    assert!(parse_duration("").is_err());
}

#[test]
fn test_error_message_quality() {
    let err = parse_duration("10x").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("10x"));
    assert!(msg.contains("s") || msg.contains("m") || msg.contains("h") || msg.contains("d"));
}
