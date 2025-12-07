//! The data broker.

// TODO: Rework module visibility, nesting, public exports.

// Currently, `tokio` is only used by tests. It will be used more later, so instead of making
// it a test-only dependency for the time being, this is added temporarily to suppress warnings.
// TODO: Remove.
use tokio as _;

pub mod errors;

pub mod connector;

pub mod rest;

pub mod encode;
