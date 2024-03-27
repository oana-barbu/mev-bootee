use std::prelude::v1::*;

use eth_types::{SH160, Transaction};
use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub enum MevBooTeeMode {
    ProposerAide,
    BuilderAide,
    Assembler,
    FullTeeBuilder,
}

#[derive(Debug, thiserror::Error)]
pub enum MevBooTeeError {
    #[error("bad signed header object: {0}")]
    BadSignedHeader(String)
}
