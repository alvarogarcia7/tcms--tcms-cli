//! Layer 3: load, select (later: model, render) for `test-executor report`.
//! Only `test-executor` depends on this crate.

pub mod legacy;
pub mod load;
pub mod run;
pub mod select;

pub use run::LoadedRun;
