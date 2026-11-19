use super::Timestamp;

fn at(text: &str) -> Timestamp {
    Timestamp::parse(text).unwrap_or_else(|| panic!("{text} should parse"))
}

#[test]
fn utc_texts_read_as_unix_seconds() {
    assert_eq!(at("1970-01-01T00:00:00Z").unix_seconds(), 0);
    assert_eq!(at("2023-11-14T22:13:20Z").unix_seconds(), 1_700_000_000);
    assert_eq!(at("1969-12-31T23:59:59Z").unix_seconds(), -1);
    assert_eq!(at("2024-02-29T00:00:00z").unix_seconds(), 1_709_164_800);
}

#[test]
fn one_instant_written_in_two_offsets_is_equal() {
    assert_eq!(at("2026-02-28T19:00:00-05:00"), at("2026-03-01T00:00:00Z"));
    assert_eq!(at("2026-03-01T05:30:00+05:30"), at("2026-03-01T00:00:00Z"));
    // As strings the first sorts before the second; in time they are equal.
    assert!("2026-02-28T19:00:00-05:00" < "2026-03-01T00:00:00Z");
}

#[test]
fn order_follows_time_not_spelling() {
    assert!(at("2026-03-01T00:00:00Z") < at("2026-03-01T00:00:00.5Z"));
    assert!(at("2026-03-01T00:00:00.123Z") < at("2026-03-01T00:00:00.1231Z"));
    assert_eq!(
        at("2026-03-01T00:00:00.1Z"),
        at("2026-03-01T00:00:00.100000000Z")
    );
    assert_eq!(
        at("2026-03-01T00:00:00.1234567891Z"),
        at("2026-03-01T00:00:00.123456789Z")
    );
    assert!(at("2026-02-28T23:00:00-05:00") > at("2026-03-01T03:00:00Z"));
    assert_eq!(at("2026-12-31T23:59:60Z"), at("2027-01-01T00:00:00Z"));
}

#[test]
fn anything_else_is_none() {
    for text in [
        "",
        "2026-03-01",
        "2026-03-01T00:00:00",
        "2026-03-01T00:00Z",
        "2026-03-01 00:00:00Z",
        "2026-13-01T00:00:00Z",
        "2026-02-29T00:00:00Z",
        "2026-04-31T00:00:00Z",
        "2026-03-01T24:00:00Z",
        "2026-03-01T00:60:00Z",
        "2026-03-01T00:00:61Z",
        "2026-03-01T00:00:00.Z",
        "2026-03-01T00:00:00+0500",
        "2026-03-01T00:00:00+24:00",
        "2026-03-01T00:00:00Z ",
        "2026-03-01T00:00:00GMT",
        "20260301T000000Z",
    ] {
        assert_eq!(Timestamp::parse(text), None, "{text:?}");
    }
}
