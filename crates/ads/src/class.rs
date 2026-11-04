//! Class joining. ADS uses its own class vocabulary (not Tailwind), so a plain
//! join is all that is needed — no conflict resolution.

/// Join non-empty class fragments into one space-separated string.
#[must_use]
pub fn join(parts: &[&str]) -> String {
    parts
        .iter()
        .filter(|p| !p.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Append an optional caller-supplied class to a base class.
#[must_use]
pub fn with_extra(base: &str, extra: Option<&str>) -> String {
    match extra {
        Some(e) if !e.is_empty() => format!("{base} {e}"),
        _ => base.to_owned(),
    }
}
