//! Solver type definitions re-exported from luma_layout.

pub use luma_layout::solver::{SolverSizeMode, SolverSpec};
pub use luma_layout::types::{UAlignItemsExt, UContentAlignExt};

/// Configuration fragment shared between item and container alignment code paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SolverAlignConfig {
    pub justify_items: Option<UAlignItemsExt>,
    pub align_content: Option<UContentAlignExt>,
}
