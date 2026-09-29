//! Glob re-export of the framework runtime crate `appfw_runtime`, giving the
//! rest of this crate a `crate::platform::runtime` path to the same types
//! and submodules.

#![allow(unused_imports)]
#![allow(ambiguous_glob_reexports)]

pub use appfw_runtime::*;
