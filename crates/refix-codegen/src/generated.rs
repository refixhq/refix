use crate::Warning;

/// The generated module and any [`Warning`] produced along the way.
#[derive(Debug)]
pub struct Generated {
    pub code: String,
    pub warnings: Vec<Warning>,
}
