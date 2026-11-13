//! The platform-independent engine.
//!
//! An [`RawElement`](crate::element::RawElement) description is built into a
//! [`tree`] of nodes with stable identifiers, sized and positioned by
//! [`layout`] (Taffy, with text measured through [`measure`]), and compared
//! with the previous tree by [`render`] to produce the operations a platform
//! applies. [`runtime`] ties those steps together for one host.

pub mod geometry;
pub mod layout;
pub mod measure;
pub mod render;
pub mod runtime;
pub mod tree;
