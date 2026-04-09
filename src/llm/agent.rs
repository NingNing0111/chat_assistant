use anyhow::Result;
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Prompt;
use rig::providers::openai::Client;

use super::response::ResponseSegmenter;

/// LLM Agent wrapper using rig completions API
pub struct LlmAgent {
    client: Client,
    model: String,
    segmenter: ResponseSegmenter,
}

impl LlmAgent {
    /// Create a new LLM agent with OpenAI
    pub async fn new(
        api_key: Option<&str>,
        base_url: Option<&str>,
        model: &str,
        preamble: &str,
    ) -> Result<Self> {
        let client = if let Some(key) = api_key {
            Client::builder()
                .api_key(key)
                .base_url(base_url.unwrap_or("https://api.openai.com/v1"))
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to build client: {}", e))?
        } else {
            Client::from_env()
        };

        let _agent = client.agent(model).preamble(preamble).build();

        Ok(Self {
            client,
            model: model.to_string(),
            segmenter: ResponseSegmenter::new(),
        })
    }

    /// Prompt the agent and get a response
    pub async fn prompt(&self, text: &str) -> Result<String> {
        let agent = self.client.agent(&self.model).build();
        let response = agent.prompt(text).await?;
        Ok(response)
    }

    /// Get response segments from the LLM
    pub async fn stream_segments(&self, text: &str) -> Result<Vec<String>> {
        let response = self.prompt(text).await?;
        let segments = self.segmenter.segment(&response);
        Ok(segments)
    }

    /// Get the response segmenter
    pub fn segmenter(&self) -> &ResponseSegmenter {
        &self.segmenter
    }
}