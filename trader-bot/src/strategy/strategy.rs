// Re-export the canonical Strategy trait from the core module.
// This ensures backward compatibility with existing code.
pub use crate::core::Strategy;

// Old trait kept for backward compatibility, delegates to core::Strategy
pub trait LegacyStrategy {
    async fn run<T, E>() -> Result<T, E>;
}
