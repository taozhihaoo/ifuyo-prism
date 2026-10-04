pub mod model;
pub use model::{
    canonicalize, combine_fingerprints, fingerprint_value, fnv1a_hex, CaptureStatus,
    DomainFreshness, DomainStatus, ProjectSnapshot, SnapshotBasis, SnapshotCaptureResult,
    SnapshotDomainState, SNAPSHOT_SCHEMA_VERSION,
};
