pub mod doh;
pub mod dot;
pub mod udp;

// Re-export for convenience
pub use doh::serve as serve_doh;
pub use dot::serve as serve_dot;
pub use udp::serve as serve_udp;
