//! Core library of pairdock: talks to Logitech receivers and devices over
//! HID++ using nothing but the operating system's generic HID driver.
//!
//! The protocol itself lives in the vendored [`hidpp`] crate. This crate adds
//! the HID transport (via `hidapi`), device enumeration and the higher-level
//! operations the CLI and the desktop app are built on.

pub mod config;
pub mod controls;
pub mod enumerate;
pub mod inventory;
pub mod keys;
pub mod pairing;
pub mod remap;
pub mod transport;

pub use hidpp;
