//! Style crate for Luma.
//!
//! This crate owns embedded fonts, icons, and the shared [`style::Theme`]
//! resource that widgets can read during setup.

pub mod style;

/// The recommended import surface when depending on `luma_style` directly.
pub mod prelude {
    pub use crate::style::prelude::*;
}
