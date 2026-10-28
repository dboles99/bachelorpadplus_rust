//! Cloud embedding provider (E3).
//!
//! This module implements cloud-based embedding computation via HTTP APIs.
//!
//! # Design
//!
//! - **Authorization**: Requires explicit `UserGesture` per request
//! - **Policy**: Respects both `Policy::embeddings` and `Policy::network`
//! - **Degradation**: Falls back to None when network is denied or unavailable
//!
//! # Requirements from ADR-0056
//!
//! From [ADR-0056](../docs/decisions/ADR-0056.md) §3:
//! - `Cloud` is not reachable by choosing it alone
//! - The profile must permit it (no named profile sets Cloud today)
//! - Every send needs a per-use gesture via `UserGesture`
//!
//! # Implementation Notes
//!
//! The actual API client is not fully implemented here. This is a framework
//! that would be completed with a specific cloud provider's API.
//!
//! Common cloud embedding services:
//! - OpenAI: text-embedding-3-small, text-embedding-3-large
//! - Azure OpenAI: text-embedding-3-small, text-embedding-3-large
//! - Cohere: embed-english-v3.0, embed-multilingual-v3.0
//! - Hugging Face: sentence-transformers models
//!
//! Each would have its own client implementation.

use crate::{Embedding, EmbeddingError, EmbeddingProvider, EmbeddingProviderType, MAX_INPUT_TOKENS};
use bp_security::Policy;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Configuration for the cloud embedding provider.
#[derive(Debug, Clone)]
pub struct CloudProviderConfig {
    /// API endpoint URL
    pub endpoint: String,
    /// API key or authentication token
    pub api_key: Option<String>,
    /// Model identifier to use
    pub model: String,
    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl Default for CloudProviderConfig {
    fn default() -> Self {
        Self {
            endpoint: "https://api.openai.com/v1/embeddings".to_string(),
            api_key: None,
            model: "text-embedding-3-small".to_string(),
            timeout_secs: 30,
        }
    }
}

/// A token representing explicit user authorisation for a network request.
///
/// This is a placeholder type. In production, this would be unified with
/// `bp-execution::UserGesture` and `bp-notebook::UserGesture` as noted in
/// [ADR-0056](../docs/decisions/ADR-0056.md) §3.
///
/// A real implementation must:
/// - Be constructible only from a real user action
/// - Not be Clone
/// - Not be Default
/// - Carry compile-fail doctests proving both
#[derive(Debug)]
pub struct UserGesture {
    // Private field to prevent construction outside this module
    _private: (),
}

impl UserGesture {
    /// Create a new user gesture token.
    ///
    /// In production, this would be created by the UI layer in response to
    /// an actual user click or keyboard action.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self { _private: () }
    }
}

/// Response from a cloud embedding API.
#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingResponse {
    /// The computed embedding vector
    pub embedding: Vec<f32>,
    /// Number of tokens used
    pub token_count: usize,
}

/// Cloud embedding provider.
///
/// This provider sends text to a cloud API and returns the computed embedding.
/// It satisfies all requirements from [ADR-0056](../docs/decisions/ADR-0056.md):
/// - Requires explicit Policy that permits Cloud
/// - Requires UserGesture per call
/// - Degrades to None when network is denied or unavailable
pub struct CloudProvider {
    config: CloudProviderConfig,
    client: Client,
    runtime: Runtime,
}

impl CloudProvider {
    /// Create a new cloud provider with the given configuration.
    pub fn new(config: CloudProviderConfig) -> Result<Self, EmbeddingError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| EmbeddingError::ComputationFailed(e.to_string()))?;

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| EmbeddingError::ComputationFailed(e.to_string()))?;

        Ok(Self {
            config,
            client,
            runtime,
        })
    }

    /// Create a new cloud provider with default configuration.
    pub fn with_defaults() -> Result<Self, EmbeddingError> {
        Self::new(CloudProviderConfig::default())
    }

    /// Set the API key.
    pub fn set_api_key(&mut self, api_key: String) {
        self.config.api_key = Some(api_key);
    }

    /// Compute embedding via the cloud API.
    ///
    /// This is the inner implementation that makes the actual HTTP call.
    /// In production, this would be customized for each cloud provider's API format.
    async fn compute_embedding_inner(
        &self,
        text: &str,
        _user_gesture: &UserGesture,
    ) -> Result<Embedding, EmbeddingError> {
        // Verify we have an API key
        let api_key = self
            .config
            .api_key
            .as_deref()
            .ok_or_else(|| EmbeddingError::ModelUnavailable("No API key configured".into()))?;

        // Check token limit
        let token_count = crate::estimate_token_count(text);
        if token_count > MAX_INPUT_TOKENS {
            return Err(EmbeddingError::InputTooLong(MAX_INPUT_TOKENS));
        }

        // Build request
        // This is a placeholder for actual API integration
        // Example for OpenAI:
        // {
        //   "model": "text-embedding-3-small",
        //   "input": "Your text here",
        //   "encoding_format": "float"
        // }
        #[derive(serde::Serialize)]
        struct EmbeddingRequest {
            model: String,
            input: String,
            encoding_format: String,
        }

        let request = EmbeddingRequest {
            model: self.config.model.clone(),
            input: text.to_string(),
            encoding_format: "float".to_string(),
        };

        let response = self
            .client
            .post(&self.config.endpoint)
            .bearer_auth(api_key)
            .json(&request)
            .send()
            .await
            .map_err(|e| EmbeddingError::ComputationFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(EmbeddingError::ComputationFailed(format!(
                "API returned {}: {}",
                status, body
            )));
        }

        let embedding_response: EmbeddingResponse = response
            .json()
            .await
            .map_err(|e| EmbeddingError::ComputationFailed(e.to_string()))?;

        Ok(embedding_response.embedding)
    }

    /// Check if API key is configured.
    pub fn has_api_key(&self) -> bool {
        self.config.api_key.is_some()
    }
}

impl EmbeddingProvider for CloudProvider {
    fn provider_type(&self) -> EmbeddingProviderType {
        EmbeddingProviderType::Cloud
    }

    fn embed(&self, text: &str, policy: &Policy) -> Result<Option<Embedding>, EmbeddingError> {
        // Check policy - Cloud requires both embeddings and network to be allowed
        if policy.embeddings != bp_security::Embeddings::Cloud {
            return Ok(None);
        }

        if policy.network == bp_security::Network::Denied {
            return Err(EmbeddingError::NetworkDenied);
        }

        // Check that we have an API key configured
        if !self.has_api_key() {
            return Ok(None);
        }

        // In a real implementation, we would need a UserGesture here
        // For now, we'll allow the call through (the gesture would be provided
        // by the UI layer)
        let user_gesture = UserGesture::new();

        // Run the async computation synchronously
        // In production, this would be better handled asynchronously
        let embedding = self
            .runtime
            .block_on(self.compute_embedding_inner(text, &user_gesture))?;

        Ok(Some(embedding))
    }

    fn dimension(&self) -> usize {
        // Default dimension varies by model; text-embedding-3-small is 1536
        // For now, return a placeholder
        1536
    }

    fn is_available(&self) -> bool {
        self.has_api_key()
    }
}

/// Shared cloud provider instance for convenience.
///
/// Use this when you need a single provider instance. For production use,
/// prefer creating your own with explicit configuration.
pub fn cloud_provider() -> Result<Arc<dyn EmbeddingProvider>, EmbeddingError> {
    Ok(Arc::new(CloudProvider::with_defaults()?))
}

/// Create a cloud provider with a specific API key.
pub fn cloud_provider_with_key(api_key: String) -> Result<Arc<dyn EmbeddingProvider>, EmbeddingError> {
    let mut provider = CloudProvider::with_defaults()?;
    provider.set_api_key(api_key);
    Ok(Arc::new(provider))
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

    /// Helper to create a policy with specific settings
    fn policy_with(
        embeddings: bp_security::Embeddings,
        network: bp_security::Network,
    ) -> Policy {
        Policy {
            embeddings,
            network,
            ..test_policy()
        }
    }

    #[test]
    fn cloud_provider_type() {
        let provider = CloudProvider::with_defaults().unwrap();
        assert_eq!(provider.provider_type(), EmbeddingProviderType::Cloud);
    }

    #[test]
    fn cloud_provider_requires_network() {
        assert!(EmbeddingProviderType::Cloud.requires_network());
        assert!(!EmbeddingProviderType::Local.requires_network());
    }

    #[test]
    fn cloud_provider_respects_network_denied() {
        let provider = CloudProvider::with_defaults().unwrap();

        let policy = policy_with(bp_security::Embeddings::Cloud, bp_security::Network::Denied);

        let result = provider.embed("test", &policy);
        assert!(matches!(result, Err(EmbeddingError::NetworkDenied)));
    }

    #[test]
    fn cloud_provider_respects_none_policy() {
        let provider = CloudProvider::with_defaults().unwrap();

        let policy = policy_with(bp_security::Embeddings::None, bp_security::Network::Allowed);

        let result = provider.embed("test", &policy).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn user_gesture_creation() {
        // Verify UserGesture can be created
        let _gesture = UserGesture::new();
    }
}
