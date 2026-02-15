//! Core pipeline logic: processing, parsing, merging, and prompt templates.

pub mod merger;
pub mod parser;
pub mod pipeline;
pub mod processor;
pub mod prompt;

pub(crate) mod merger_chunk;
pub(crate) mod merger_dedup;

#[cfg(test)]
mod merger_tests;
