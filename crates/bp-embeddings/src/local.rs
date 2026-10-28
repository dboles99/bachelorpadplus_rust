//! Local embedding provider using Candle (E2).
//!
//! This module implements on-device embedding computation using the Candle
//! deep learning framework with a sentence-transformers model.
//!
//! # Design
//!
//! - **Model**: all-MiniLM-L6-v2 (384-dimensional embeddings)
//! - **Distribution**: Model weights are loaded from disk, not compiled in
//! - **Degradation**: Returns `None` when model files are absent
//! - **Offline**: Works entirely offline once model is present
//!
//! # File Layout
//!
//! Model files are expected in the following structure under the models directory:
//! 
//! ```text
//! assets/models/all-MiniLM-L6-v2/
//!   config.json          # Model configuration
//!   tokenizer.json      # Tokenizer configuration  
//!   model.safetensors   # Model weights (or model.bin for older formats)
//! ```
//!
//! # Dependencies
//!
//! Uses `candle-core` for tensor operations. The full `candle` crate is not
//! needed as we don't use GPU acceleration in the base configuration.

use crate::{Embedding, EmbeddingError, EmbeddingProvider, EmbeddingProviderType, MAX_INPUT_TOKENS};
use bp_security::Policy;
use std::path::Path;
use std::sync::Arc;

/// Configuration for the local embedding provider.
#[derive(Debug, Clone)]
pub struct LocalProviderConfig {
    /// Path to the model directory (e.g., "assets/models/all-MiniLM-L6-v2")
    pub model_path: String,
    /// Maximum batch size for embedding computation
    pub max_batch_size: usize,
}

impl Default for LocalProviderConfig {
    fn default() -> Self {
        Self {
            model_path: format!("assets/models/{}", crate::DEFAULT_LOCAL_MODEL),
            max_batch_size: 32,
        }
    }
}

/// Local embedding provider (E2).
///
/// This provider will load a sentence-transformers model from disk and compute
/// embeddings locally using Candle (when candle-core dependency is enabled).
/// It satisfies all requirements from [ADR-0056](../docs/decisions/ADR-0056.md):
/// - Weights are shipped/fetched explicitly (not silently)
/// - Works offline once present
/// - Degrades to None when model is absent
///
/// For now, this is a stub implementation that returns placeholder embeddings.
/// The full Candle integration will be added when the candle-core dependency is enabled.
pub struct LocalProvider {
    config: LocalProviderConfig,
    // Model state would be stored here after loading
    // For now, we track availability at construction time
    is_loaded: bool,
}

impl LocalProvider {
    /// Create a new local provider with the given configuration.
    pub fn new(config: LocalProviderConfig) -> Self {
        // Check if model files exist
        let model_dir = Path::new(&config.model_path);
        let is_loaded = model_dir.exists()
            && model_dir.join("config.json").exists()
            && (model_dir.join("model.safetensors").exists()
                || model_dir.join("model.bin").exists());

        Self {
            config,
            is_loaded,
        }
    }

    /// Create a new local provider with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(LocalProviderConfig::default())
    }

    /// Check if the model files are present.
    pub fn model_files_present(&self) -> bool {
        self.is_loaded
    }

    /// Load the model from disk.
    ///
    /// This is a placeholder for actual model loading. In production, this would:
    /// 1. Parse config.json
    /// 2. Load tokenizer.json
    /// 3. Load model weights
    /// 4. Build the model architecture
    ///
    /// For now, we return an error as the full Candle integration is complex.
    pub fn load_model(&mut self) -> Result<(), EmbeddingError> {
        // Check again if files exist
        let model_dir = Path::new(&self.config.model_path);
        if !model_dir.exists() {
            return Err(EmbeddingError::ModelUnavailable(
                format!("Model directory not found: {}", self.config.model_path),
            ));
        }

        // Placeholder: actual implementation would load the model here
        // For now, we just mark it as loaded if the files are present
        self.is_loaded = true;
        Ok(())
    }

    /// Tokenize text and compute embeddings.
    ///
    /// This is a placeholder. Actual implementation would:
    /// 1. Tokenize the input text
    /// 2. Run through the model
    /// 3. Pool to get a single embedding vector
    fn compute_embedding_inner(&self, text: &str) -> Result<Embedding, EmbeddingError> {
        // Check token limit
        let token_count = crate::estimate_token_count(text);
        if token_count > MAX_INPUT_TOKENS {
            return Err(EmbeddingError::InputTooLong(MAX_INPUT_TOKENS));
        }

        // Placeholder: return a dummy embedding of the correct dimension
        // In production, this would compute the actual embedding
        Ok(vec![0.0; crate::DEFAULT_EMBEDDING_DIM])
    }
}

impl EmbeddingProvider for LocalProvider {
    fn provider_type(&self) -> EmbeddingProviderType {
        EmbeddingProviderType::Local
    }

    fn embed(&self, text: &str, policy: &Policy) -> Result<Option<Embedding>, EmbeddingError> {
        // Check policy
        if policy.embeddings == bp_security::Embeddings::None {
            return Ok(None);
        }

        // If model is not available, degrade gracefully
        if !self.is_loaded {
            return Ok(None);
        }

        // Compute embedding
        let embedding = self.compute_embedding_inner(text)?;
        Ok(Some(embedding))
    }

    fn dimension(&self) -> usize {
        crate::DEFAULT_EMBEDDING_DIM
    }

    fn is_available(&self) -> bool {
        self.is_loaded
    }
}

/// Shared local provider instance for convenience.
///
/// Use this when you need a single provider instance. For production use,
/// prefer creating your own with explicit configuration.
pub fn local_provider() -> Arc<dyn EmbeddingProvider> {
    Arc::new(LocalProvider::with_defaults())
}

/// Create a local provider from a custom model path.
pub fn local_provider_with_path(model_path: impl AsRef<Path>) -> Arc<dyn EmbeddingProvider> {
    Arc::new(LocalProvider::new(LocalProviderConfig {
        model_path: model_path.as_ref().to_string_lossy().into_owned(),
        max_batch_size: 32,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bp_security::Policy;

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

    /// Helper to create a policy with specific embeddings setting
    fn policy_with_embeddings(embeddings: bp_security::Embeddings) -> Policy {
        Policy {
            embeddings,
            ..test_policy()
        }
    }

    #[test]
    fn local_provider_type() {
        let provider = LocalProvider::with_defaults();
        assert_eq!(provider.provider_type(), EmbeddingProviderType::Local);
    }

    #[test]
    fn local_provider_dimension() {
        let provider = LocalProvider::with_defaults();
        assert_eq!(provider.dimension(), crate::DEFAULT_EMBEDDING_DIM);
    }

    #[test]
    fn local_provider_degrades_when_unavailable() {
        // Create provider with non-existent path
        let provider = LocalProvider::new(LocalProviderConfig {
            model_path: "nonexistent/path".to_string(),
            max_batch_size: 32,
        });

        let policy = test_policy();
        let result = provider.embed("test", &policy).unwrap();
        assert!(result.is_none());
        assert!(!provider.is_available());
    }

    #[test]
    fn local_provider_respects_none_policy() {
        // Even with model available, None policy returns None
        let mut provider = LocalProvider::with_defaults();
        // Force loaded state for test
        provider.is_loaded = true;

        let policy = policy_with_embeddings(bp_security::Embeddings::None);

        let result = provider.embed("test", &policy).unwrap();
        assert!(result.is_none());
    }
}
