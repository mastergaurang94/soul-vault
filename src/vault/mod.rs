//! Vault I/O: configuration, reading, writing, and source tracking.

pub mod chatgpt;
pub mod config;
pub mod local;
pub mod read;
pub mod sources;
pub mod write;

pub(crate) mod chatgpt_detect;
pub(crate) mod chatgpt_format;
pub(crate) mod chatgpt_parse;
pub(crate) mod chatgpt_types;

pub(crate) mod local_discover;
pub(crate) mod local_extract;

pub(crate) mod sources_classify;
pub(crate) mod sources_store;
pub(crate) mod sources_types;

pub(crate) mod write_digest;
pub(crate) mod write_entries;
pub(crate) mod write_slug;
pub(crate) mod write_types;

#[cfg(test)]
mod chatgpt_misc_tests;
#[cfg(test)]
mod chatgpt_parse_more_tests;
#[cfg(test)]
mod chatgpt_parse_tests;
#[cfg(test)]
mod config_tests;
#[cfg(test)]
mod local_tests;
#[cfg(test)]
mod sources_tests;
#[cfg(test)]
mod write_tests;
