//! Interaction crate for Luma.
//!
//! This crate provides pointer-driven picking, interaction state tracking, and
//! hover/press feedback helpers that integrate with Luma roots and widgets.

pub mod interaction;

/// The recommended import surface when depending on `luma_interaction` directly.
pub mod prelude {
    pub use crate::interaction::prelude::*;
}
