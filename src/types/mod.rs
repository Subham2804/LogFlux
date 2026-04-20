//! Core types for the ingest → normalize → summarize → delivery pipeline.

/// A raw log line (or blob) as received from a source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEvent {
    /// Unparsed payload.
    pub raw: String,
    /// Logical source label (file path, `"stdin"`, listener id, etc.).
    pub source: String,
    /// Category of the event.
    pub category: String,
    /// Milliseconds since Unix epoch when the event was ingested.
    pub received_at_ms: i64,
}

impl LogEvent {
    pub fn new(raw: impl Into<String>, source: impl Into<String>, category: impl Into<String>, received_at_ms: i64) -> Self {
        Self {
            raw: raw.into(),
            source: source.into(),
            category: category.into(),
            received_at_ms,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_event_new() {
        let e = LogEvent::new("line", "stdin", "test", 1_700_000_000_000);
        assert_eq!(e.raw, "line");
        assert_eq!(e.source, "stdin");
        assert_eq!(e.category, "test");
        assert_eq!(e.received_at_ms, 1_700_000_000_000);
    }

}