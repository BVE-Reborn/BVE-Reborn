//! C API for BVE-Reborn high performance libraries.

// Rust warnings
#![allow(unsafe_code)] // We're in an ffi
#![warn(unused)]
#![deny(nonstandard_style)]
#![deny(future_incompatible)]
#![deny(rust_2018_idioms)]
// Clippy warnings
#![warn(clippy::cargo)]
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]
#![allow(clippy::cast_sign_loss)] // Annoying
#![allow(clippy::cast_precision_loss)] // Annoying
#![allow(clippy::cast_possible_truncation)] // Annoying
#![allow(clippy::cognitive_complexity)] // This is dumb
#![allow(clippy::multiple_crate_versions)] // Dependencies are hard

// Clippy Restrictions
#![warn(clippy::clone_on_ref_ptr)]
#![warn(clippy::dbg_macro)]
#![warn(clippy::get_unwrap)]
#![warn(clippy::multiple_inherent_impl)]
#![warn(clippy::option_unwrap_used)]
#![warn(clippy::print_stdout)]
#![warn(clippy::result_unwrap_used)]
#![warn(clippy::unimplemented)]
#![warn(clippy::wildcard_enum_match_arm)]
#![warn(clippy::wrong_pub_self_convention)]
