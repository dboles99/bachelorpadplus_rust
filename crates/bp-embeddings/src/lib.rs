//! Embedding providers for Phase 10 semantic search (E1, E2, E3).
//!
//! This crate provides the embedding computation layer that sits between
//! `bp-storage` (which stores and queries embeddings) and the actual embedding
//! models or services.
//!
//! # Architecture
//!
//! - **E1** (done): `bp-storage` has the store and query infrastructure
//! - **E2**: `LocalProvider` computes embeddings on-device using Candle
//! - **E3**: `CloudProvider` computes embeddings via a remote API
//!
//! # Dependencies
//!
//! - **Local provider**: Uses `candle-core` for pure-Rust tensor operations
//!   - Model weights are loaded from disk, not compiled in
//!   - Degrades gracefully when model files are absent
//! - **Cloud provider**: Uses `reqwest` for HTTP calls to embedding services
//!   - Requires a `UserGesture` to authorise each request
//!   - Respects `Policy::network` and `Policy::embeddings`
//!
//! # Design Decisions
//!
//! - **No automatic network access**: Cloud embeddings require explicit
//!   `UserGesture` per call, per [ADR-0033](../docs/decisions/ADR-0033.md) and
//!   [ADR-0056](../docs/decisions/ADR-0056.md) §3.
//! - **Profile is a ceiling**: User preference can tighten but not loosen
//!   the effective policy, per [ADR-0056](../docs/decisions/ADR-0056.md) §1.
//! - **Degradation to None**: Both providers fall back to returning no
//!   embedding when unavailable, per [ADR-0056](../docs/decisions/ADR-0056.md) §2.

#![forbid(unsafe_code)]

use bp_security::Embeddings as EmbeddingsPolicy;
use bp_security::Network as NetworkPolicy;
use bp_security::Policy;
use std::sync::Arc;
use thiserror::Error;

pub mod cloud;
pub mod local;

/// Crate identity used by workspace smoke tests and diagnostics.
pub const CRATE_NAME: &str = "bp-embeddings";

/// The default embedding dimension for the bundled local model.
/// all-MiniLM-L6-v2 produces 384-dimensional embeddings.
pub const DEFAULT_EMBEDDING_DIM: usize = 384;

/// The model identifier for the default local embedding model.
pub const DEFAULT_LOCAL_MODEL: &str = "all-MiniLM-L6-v2";

/// A vector of embedding coefficients.
pub type Embedding = Vec<f32>;

/// Error type for embedding operations.
#[derive(Debug, Error)]
pub enum EmbeddingError {
    /// The embedding model or configuration is not available.
    #[error("Embedding model not available: {0}")]
    ModelUnavailable(String),

    /// The requested embedding source is disabled by policy.
    #[error("Embedding computation disabled by security policy")]
    DisabledByPolicy,

    /// Network access is required but denied by policy.
    #[error("Network access denied by security policy")]
    NetworkDenied,

    /// The input text is too long for the model.
    #[error("Input text exceeds maximum token limit ({0} tokens)")]
    InputTooLong(usize),

    /// Failed to compute embedding for technical reasons.
    #[error("Embedding computation failed: {0}")]
    ComputationFailed(String),

    /// IO error accessing model files.
    #[error("IO error: {0}")]
    IoError(String),
}

impl Clone for EmbeddingError {
    fn clone(&self) -> Self {
        match self {
            Self::ModelUnavailable(s) => Self::ModelUnavailable(s.clone()),
            Self::DisabledByPolicy => Self::DisabledByPolicy,
            Self::NetworkDenied => Self::NetworkDenied,
            Self::InputTooLong(n) => Self::InputTooLong(*n),
            Self::ComputationFailed(s) => Self::ComputationFailed(s.clone()),
            Self::IoError(s) => Self::IoError(s.clone()),
        }
    }
}

/// Trait for embedding providers.
///
/// Implementations must respect the effective policy, which is the maximum
/// of the document's policy and any session override.
pub trait EmbeddingProvider: Send + Sync {
    /// Returns the type identifier for this provider.
    fn provider_type(&self) -> EmbeddingProviderType;

    /// Compute an embedding for the given text.
    ///
    /// # Arguments
    /// * `text` - The text to embed
    /// * `policy` - The effective security policy
    /// * `user_gesture` - Optional user gesture authorising network access
    ///
    /// # Returns
    /// * `Ok(Some(embedding))` - Embedding computed successfully
    /// * `Ok(None)` - Embedding unavailable (provider not configured, model missing)
    /// * `Err` - Embedding computation failed or is denied by policy
    fn embed(&self, text: &str, policy: &Policy) -> Result<Option<Embedding>, EmbeddingError>;

    /// Returns the embedding dimension this provider produces.
    fn dimension(&self) -> usize;

    /// Check if this provider is available (model loaded, API configured, etc.).
    fn is_available(&self) -> bool;
}

/// Type identifier for embedding providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EmbeddingProviderType {
    /// Local on-device embedding using Candle
    Local,
    /// Cloud-based embedding via API
    Cloud,
}

impl EmbeddingProviderType {
    /// Returns true if this provider type requires network access.
    pub fn requires_network(&self) -> bool {
        matches!(self, EmbeddingProviderType::Cloud)
    }
}

/// A no-op provider that always returns None.
///
/// Used when embeddings are disabled by policy or when no real provider is configured.
pub struct NullProvider;

impl EmbeddingProvider for NullProvider {
    fn provider_type(&self) -> EmbeddingProviderType {
        // Arbitrarily choose Local; the type doesn't matter since we never produce embeddings
        EmbeddingProviderType::Local
    }

    fn embed(&self, _text: &str, _policy: &Policy) -> Result<Option<Embedding>, EmbeddingError> {
        Ok(None)
    }

    fn dimension(&self) -> usize {
        DEFAULT_EMBEDDING_DIM
    }

    fn is_available(&self) -> bool {
        false
    }
}

/// Select the appropriate provider based on policy.
///
/// Returns a provider that respects the given policy:
/// - If policy embeddings is `None`, returns a NullProvider
/// - If policy embeddings is `Local`, returns a LocalProvider (if available)
/// - If policy embeddings is `Cloud`, returns a CloudProvider (if network allowed)
///
/// The caller is responsible for providing the actual provider implementations.
pub fn provider_for_policy(
    embeddings_policy: EmbeddingsPolicy,
    network_policy: NetworkPolicy,
    local_provider: Option<Arc<dyn EmbeddingProvider>>,
    cloud_provider: Option<Arc<dyn EmbeddingProvider>>,
) -> Arc<dyn EmbeddingProvider> {
    match embeddings_policy {
        EmbeddingsPolicy::None => Arc::new(NullProvider),
        EmbeddingsPolicy::Local => {
            local_provider.unwrap_or_else(|| Arc::new(NullProvider))
        }
        EmbeddingsPolicy::Cloud => {
            if network_policy == NetworkPolicy::Allowed {
                cloud_provider.unwrap_or_else(|| Arc::new(NullProvider))
            } else {
                // Network denied, fall back to Local if available, else None
                local_provider.unwrap_or_else(|| Arc::new(NullProvider))
            }
        }
    }
}

/// Maximum input length in tokens for the default model.
/// all-MiniLM-L6-v2 has a 512-token limit.
pub const MAX_INPUT_TOKENS: usize = 512;

/// Estimate token count for text (approximate, for length checking).
/// This is a simple whitespace-based estimate; a real tokenizer would be more accurate.
pub fn estimate_token_count(text: &str) -> usize {
    text.split_whitespace().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a test policy
    fn test_policy() -> Policy {
        Policy {
            recovery: bp_security::Recovery::Plaintext,
            clipboard: bp_security::Clipboard::InMemory,
            metadata: bp_security::Metadata::Summary,
            embeddings: bp_security::Embeddings::Local,
            network: bp_security::Network::Allowed,
            temporary_files: bp_security::TemporaryFiles::Allowed,
            zeroise: bp_security::Zeroise::Off,
        }
    }

    #[test]
    fn null_provider_always_returns_none() {
        let provider = NullProvider;
        let policy = test_policy();
        let result = provider.embed("test", &policy).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn null_provider_not_available() {
        let provider = NullProvider;
        assert!(!provider.is_available());
    }

    #[test]
    fn provider_for_policy_returns_null_for_none() {
        let provider = provider_for_policy(
            EmbeddingsPolicy::None,
            NetworkPolicy::Allowed,
            None,
            None,
        );
        let result = provider.embed("test", &test_policy()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn estimate_token_count_basic() {
        assert_eq!(estimate_token_count("hello world"), 2);
        assert_eq!(estimate_token_count(""), 0);
        assert_eq!(estimate_token_count("a b c d"), 4);
    }
}
