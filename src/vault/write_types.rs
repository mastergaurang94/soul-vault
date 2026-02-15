//! Public return types for vault writing operations.

#[derive(Debug)]
pub struct WriteResult {
    pub topics_written: Vec<String>,
    pub people_written: Vec<String>,
}
