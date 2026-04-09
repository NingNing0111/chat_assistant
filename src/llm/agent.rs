use anyhow::Result;
use async_stream::stream;
use futures_util::StreamExt;
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Prompt;
use rig::providers::openai::Client;
use rig::streaming::StreamingPrompt;

use super::response::ResponseSegmenter;

/// LLM Agent wrapper using rig completions API
pub struct LlmAgent {
    client: Client,
    model: String,
    preamble: String,
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

        Ok(Self {
            client,
            model: model.to_string(),
            preamble: preamble.to_string(),
        })
    }

    /// Prompt the agent and get a response (non-streaming)
    pub async fn prompt(&self, text: &str) -> Result<String> {
        let agent = self
            .client
            .agent(&self.model)
            .preamble(&self.preamble)
            .build();
        let response = agent.prompt(text).await?;
        Ok(response)
    }

    /// Stream response tokens one by one
    pub fn stream_tokens(&self, text: String) -> impl StreamExt<Item = Result<String>> + '_ {
        let client = self.client.clone();
        let model = self.model.clone();
        let preamble = self.preamble.clone();

        stream! {
            let agent = client.agent(&model).preamble(&preamble).build();

            let mut stream = agent.stream_prompt(text).await;

            while let Some(result) = stream.next().await {
                match result {
                    Ok(chunk) => {
                        use rig::agent::MultiTurnStreamItem;
                        use rig::streaming::StreamedAssistantContent;
                        match chunk {
                            MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(t)) => {
                                println!("{:?}", t.text);
                                yield Ok(t.text);
                            }
                            MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Final(_)) => {
                                // Stream complete
                            }
                            _ => {
                                // Ignore tool calls, reasoning, etc. for TTS
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(anyhow::anyhow!("Stream error: {}", e));
                        break;
                    }
                }
            }
        }
    }

    /// Get response segmenter (for backward compatibility)
    pub fn segmenter(&self) -> ResponseSegmenter {
        ResponseSegmenter::new()
    }
}
