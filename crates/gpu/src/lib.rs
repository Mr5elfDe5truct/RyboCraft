//! RyboCraft's GPU work on top of the shared viewport renderer (`rybocraft_viewport_gpu`): the
//! path tracer ([`path`]) and RyboCraft's GPU environment ([`env`]), which the app reads and passes
//! to the viewport's GPU code.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
#![forbid(unsafe_code)]

#[cfg(not(target_arch = "wasm32"))]
pub mod env;
#[cfg(not(target_arch = "wasm32"))]
pub mod path;

#[cfg(test)]
mod tests;
