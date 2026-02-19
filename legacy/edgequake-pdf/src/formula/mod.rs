//! Math formula detection and LaTeX conversion.
//!
//! This module provides:
//! - Symbol map for Unicode math to LaTeX conversion
//! - Formula detection based on math symbol density
//! - LaTeX reconstruction from block structure
//!
//! # Architecture
//!
//! The formula detection works in three stages:
//! 1. **Symbol Detection**: Identify math symbols in text using symbol_map
//! 2. **Density Calculation**: Calculate math symbol density per block
//! 3. **LaTeX Reconstruction**: Convert detected formulas to LaTeX
//!
//! # Example
//!
//! ```rust,ignore
//! use edgequake_pdf::formula::{FormulaDetector, FormulaConfig};
//! use edgequake_pdf::Page;
//!
//! let detector = FormulaDetector::new(FormulaConfig::default());
//! let formulas = detector.detect_formulas(&page);
//! for formula in formulas {
//!     println!("LaTeX: {}", formula.latex);
//! }
//! ```

mod detector;
mod symbol_map;

pub use detector::{Formula, FormulaConfig, FormulaDetector};
pub use symbol_map::{SymbolMap, MATH_SYMBOL_MAP};
