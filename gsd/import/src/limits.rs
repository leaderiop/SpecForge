//! The import's caps (ADR 0391 D9): past either, the plan is refused "truncated at N", never answered in part.

/// The most operations one plan holds: 13-02 measured one plan of 2,000 creates applying in about 38 s, and the host's
/// apply cost grows about quadratically (5,000 took 218 s).
pub const MAX_OPS: usize = 2_000;

/// The most report entries: ten per operation, room for every file, section and row of a folder at the op cap.
pub const MAX_ENTRIES: usize = 10 * MAX_OPS;
