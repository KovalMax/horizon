pub fn canonicalize_legacy_alias(time_zone: &str) -> &str {
    match time_zone {
        "Europe/Kiev" => "Europe/Kyiv",
        _ => time_zone,
    }
}
