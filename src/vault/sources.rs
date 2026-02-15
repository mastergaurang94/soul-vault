//! Source tracking public API.

#[allow(unused_imports)]
pub use crate::vault::sources_classify::{
    classify_files, compute_file_hash, update_source_tracking,
};
#[allow(unused_imports)]
pub use crate::vault::sources_store::{
    get_source_stats, read_sources, sources_config_path, write_sources,
};
#[allow(unused_imports)]
pub use crate::vault::sources_types::{
    IngestClassification, SourceEntry, SourceSummary, SourcesConfig,
};
