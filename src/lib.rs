#![feature(never_type)]
#![feature(type_changing_struct_update)]
#![feature(adt_const_params)]
// TODO: Can the lint be allowed for only this attribute?
#![feature(unsized_const_params)]
#![expect(incomplete_features, reason = "Required for `unsized_const_params`.")]

//! The data broker.

// TODO: Rework module visibility, nesting, public exports.

// Currently, `tokio` is only used by tests. It will be used more later, so instead of making
// it a test-only dependency for the time being, this is added temporarily to suppress warnings.
// TODO: Remove.
use tokio as _;

#[allow(unused_imports)]
// Silence unused-crate-dependencies for dev-dependency `trybuild`
#[cfg(test)]
use trybuild as _;

pub mod errors;

pub mod connector;

pub mod rest;

pub mod encode;

pub mod query;
