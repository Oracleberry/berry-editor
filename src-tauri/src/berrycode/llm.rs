//! LLM API client integration

use crate::berrycode::Result;
use crate::berrycode::models::Model;
use crate::berrycode::tools::{Tool, ToolCall};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use anyhow::anyhow;
use futures::stream::StreamExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl Message {
    pub fn user(content: String) -> Self {
        Self {
            role: "user".to_string(),
            content: Some(content),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant(content: String) -> Self {
        Self {
            role: "assistant".to_string(),
            content: Some(content),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant_with_tools(tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: None,
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }

    pub fn tool(tool_call_id: String, content: String) -> Self {
        Self {
            role: "tool".to_string(),
            content: Some(content),
            tool_calls: None,
            tool_call_id: Some(tool_call_id),
        }
    }

    pub fn system(content: String) -> Self {
        Self {
            role: "system".to_string(),
            content: Some(content),
            tool_calls: None,
            tool_call_id: None,
        }
    }
}

#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Tool>>,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct StreamChoice {
    delta: Delta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Delta {
    content: Option<String>,
    tool_calls: Option<Vec<ToolCall>>,
}

/// Response from LLM (supports both text and tool calls)
#[derive(Debug, Clone)]
pub enum LLMResponse {
    Text(String),
    ToolCalls(Vec<ToolCall>),
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: usize,
    completion_tokens: usize,
    #[allow(dead_code)]
    total_tokens: usize,
}

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<Vec<AnthropicContent>>,
    messages: Vec<AnthropicMessage>,
    max_tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Serialize, Clone)]
struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicContent>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AnthropicContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_control: Option<CacheControl>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct CacheControl {
    #[serde(rename = "type")]
    cache_type: String,  // "ephemeral"
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: usize,
    output_tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_creation_input_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_read_input_tokens: Option<usize>,
}

pub struct LLMClient {
    client: Client,
    api_key: String,
    api_base: String,
    model: String,
    provider: LLMProvider,
}

#[derive(Debug, Clone)]
pub enum LLMProvider {
    OpenAI,
    Anthropic,
    OpenRouter,
    Custom,
}

impl LLMClient {
    pub fn new(model: &Model, api_key: String) -> Result<Self> {
        let provider = Self::detect_provider(&model.name);
        let mut api_base = Self::get_api_base(&provider);

        // Override with environment variable if set
        if let Ok(env_base) = std::env::var("OPENAI_API_BASE") {
            api_base = env_base;
        }

        Ok(Self {
            client: Client::new(),
            api_key,
            api_base,
            model: model.name.clone(),
            provider,
        })
    }

    fn detect_provider(model_name: &str) -> LLMProvider {
        if model_name.contains("gpt") || model_name.contains("o1") || model_name.contains("o3") || model_name.contains("deepseek") {
            LLMProvider::OpenAI
        } else if model_name.contains("claude") || model_name.contains("sonnet") || model_name.contains("opus") {
            LLMProvider::Anthropic
        } else {
            LLMProvider::Custom
        }
    }

    fn get_api_base(provider: &LLMProvider) -> String {
        match provider {
            LLMProvider::OpenAI => "https://api.openai.com/v1".to_string(),
            LLMProvider::Anthropic => "https://api.anthropic.com/v1".to_string(),
            LLMProvider::OpenRouter => "https://openrouter.ai/api/v1".to_string(),
            LLMProvider::Custom => "http://KyosukenoMac-Studio.local:11434/api/generate".to_string(), // Ollama on KyosukenoMac-Studio.local
        }
    }

    pub async fn chat(&self, messages: Vec<Message>) -> Result<(String, usize, usize)> {
        match self.provider {
            LLMProvider::OpenAI | LLMProvider::Custom | LLMProvider::OpenRouter => {
                self.chat_openai_format(messages, None).await
            }
            LLMProvider::Anthropic => self.chat_anthropic(messages, None).await,
        }
    }

    /// Chat with optional prefill (forces AI to start response with specific text)
    /// Only works with Anthropic provider
    pub async fn chat_with_prefill(&self, messages: Vec<Message>, prefill: Option<String>) -> Result<(String, usize, usize)> {
        match self.provider {
            LLMProvider::Anthropic => self.chat_anthropic(messages, prefill).await,
            _ => {
                tracing::warn!("Prefill is only supported for Anthropic provider, ignoring");
                self.chat(messages).await
            }
        }
    }

    /// Chat with tool support
    pub async fn chat_with_tools(&self, messages: Vec<Message>, tools: Vec<Tool>) -> Result<(LLMResponse, usize, usize)> {
        match self.provider {
            LLMProvider::OpenAI | LLMProvider::Custom | LLMProvider::OpenRouter => {
                self.chat_openai_with_tools(messages, tools).await
            }
            LLMProvider::Anthropic => {
                // For now, Anthropic doesn't support tools in this implementation
                // Fall back to regular chat without prefill
                let (text, input, output) = self.chat_anthropic(messages, None).await?;
                Ok((LLMResponse::Text(text), input, output))
            }
        }
    }

    async fn chat_openai_format(&self, messages: Vec<Message>, tools: Option<Vec<Tool>>) -> Result<(String, usize, usize)> {
        let request = OpenAIRequest {
            model: self.model.clone(),
            messages,
            temperature: Some(0.1),  // Slightly increased for better reasoning while maintaining consistency
            max_tokens: Some(8192),  // Increased for longer, more detailed responses
            stream: Some(false),  // Explicitly set to false (will enable streaming later)
            tools,
        };

        let url = format!("{}/chat/completions", self.api_base);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(anyhow!("API request failed with status {}: {}", status, text));
        }

        let response_data: OpenAIResponse = response.json().await?;

        let message = &response_data
            .choices
            .first()
            .ok_or_else(|| anyhow!("No choices in response"))?
            .message;

        let content = message.content.clone().unwrap_or_default();

        let (prompt_tokens, completion_tokens) = if let Some(usage) = response_data.usage {
            (usage.prompt_tokens, usage.completion_tokens)
        } else {
            (0, 0)
        };

        Ok((content, prompt_tokens, completion_tokens))
    }

    async fn chat_openai_with_tools(&self, messages: Vec<Message>, tools: Vec<Tool>) -> Result<(LLMResponse, usize, usize)> {
        let request = OpenAIRequest {
            model: self.model.clone(),
            messages,
            temperature: Some(0.1),  // Slightly increased for better reasoning while maintaining consistency
            max_tokens: Some(8192),  // Increased for longer, more detailed responses
            stream: Some(false),  // Explicitly set to false (will enable streaming later)
            tools: Some(tools),
        };

        let url = format!("{}/chat/completions", self.api_base);
        tracing::debug!("API Request URL: {}", url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(anyhow!("API request failed with status {}: {}", status, text));
        }

        let response_data: OpenAIResponse = response.json().await?;

        let message = response_data
            .choices
            .first()
            .ok_or_else(|| anyhow!("No choices in response"))?
            .message
            .clone();

        let (prompt_tokens, completion_tokens) = if let Some(usage) = response_data.usage {
            (usage.prompt_tokens, usage.completion_tokens)
        } else {
            (0, 0)
        };

        // Check if the response contains tool calls
        if let Some(tool_calls) = message.tool_calls {
            Ok((LLMResponse::ToolCalls(tool_calls), prompt_tokens, completion_tokens))
        } else if let Some(content) = message.content {
            Ok((LLMResponse::Text(content), prompt_tokens, completion_tokens))
        } else {
            Err(anyhow!("Response has neither content nor tool_calls"))
        }
    }

    async fn chat_anthropic(&self, messages: Vec<Message>, prefill: Option<String>) -> Result<(String, usize, usize)> {
        // Separate system messages from user/assistant messages
        let mut system_content: Vec<AnthropicContent> = Vec::new();
        let mut anthropic_messages: Vec<AnthropicMessage> = Vec::new();

        for msg in messages {
            if msg.role == "system" {
                // System messages go into separate system field with cache control
                if let Some(content) = msg.content {
                    system_content.push(AnthropicContent {
                        content_type: "text".to_string(),
                        text: content,
                        cache_control: Some(CacheControl {
                            cache_type: "ephemeral".to_string(),
                        }),
                    });
                }
            } else {
                // User/assistant messages
                let content = if let Some(text) = msg.content {
                    vec![AnthropicContent {
                        content_type: "text".to_string(),
                        text,
                        cache_control: None,
                    }]
                } else {
                    vec![]
                };

                anthropic_messages.push(AnthropicMessage {
                    role: msg.role,
                    content,
                });
            }
        }

        // Add prefill message if provided
        // This forces the AI to start its response with the specified text
        if let Some(ref prefill_text) = prefill {
            tracing::info!("🎯 PREFILL: Forcing AI to start with: {:?}", prefill_text);
            anthropic_messages.push(AnthropicMessage {
                role: "assistant".to_string(),
                content: vec![AnthropicContent {
                    content_type: "text".to_string(),
                    text: prefill_text.clone(),
                    cache_control: None,
                }],
            });
        }

        let request = AnthropicRequest {
            model: self.model.clone(),
            system: if system_content.is_empty() { None } else { Some(system_content) },
            messages: anthropic_messages,
            max_tokens: 4096,
            temperature: Some(0.0),
        };

        let url = format!("{}/messages", self.api_base);

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("anthropic-beta", "prompt-caching-2024-07-31")  // Enable prompt caching
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(anyhow!("API request failed with status {}: {}", status, text));
        }

        let response_data: AnthropicResponse = response.json().await?;

        let mut content = response_data
            .content
            .iter()
            .filter(|block| block.block_type == "text")
            .map(|block| block.text.clone())
            .collect::<Vec<_>>()
            .join("\n");

        // If prefill was used, prepend it to the response
        // The AI continues from the prefill, so we need to include it in the final output
        if let Some(prefill_text) = prefill {
            content = format!("{}{}", prefill_text, content);
            tracing::info!("✅ PREFILL: Response starts with forced prefix");
        }

        // Log cache performance metrics
        let (input_tokens, output_tokens) = if let Some(usage) = &response_data.usage {
            if let Some(cache_creation) = usage.cache_creation_input_tokens {
                tracing::info!("💾 Prompt Cache CREATED: {} tokens", cache_creation);
            }
            if let Some(cache_read) = usage.cache_read_input_tokens {
                tracing::info!("⚡ Prompt Cache HIT: {} tokens (90% faster!)", cache_read);
            }
            (usage.input_tokens, usage.output_tokens)
        } else {
            (0, 0)
        };

        Ok((content, input_tokens, output_tokens))
    }

    pub fn set_api_base(&mut self, base: String) {
        self.api_base = base;
    }

    /// Chat with tools and streaming support
    /// Callback will be called for each text chunk received
    pub async fn chat_with_tools_stream<F>(
        &self,
        messages: Vec<Message>,
        tools: Vec<Tool>,
        mut on_chunk: F,
    ) -> Result<(LLMResponse, usize, usize)>
    where
        F: FnMut(&str) -> () + Send,
    {
        let request = OpenAIRequest {
            model: self.model.clone(),
            messages,
            temperature: Some(0.1),  // Slightly increased for better reasoning while maintaining consistency
            max_tokens: Some(8192),  // Increased for longer, more detailed responses
            stream: Some(true),  // Enable streaming
            tools: Some(tools),
        };

        let url = format!("{}/chat/completions", self.api_base);
        tracing::debug!("API Request URL (streaming): {}", url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(anyhow!("API request failed with status {}: {}", status, text));
        }

        let mut full_content = String::new();
        let mut tool_calls_accumulator: std::collections::HashMap<usize, (String, String, String)> = std::collections::HashMap::new();
        let mut bytes_stream = response.bytes_stream();

        while let Some(chunk) = bytes_stream.next().await {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);

            // Parse SSE format: "data: {...}\n\n"
            for line in text.lines() {
                if line.starts_with("data: ") {
                    let json_str = &line[6..]; // Remove "data: " prefix

                    if json_str == "[DONE]" {
                        break;
                    }

                    // Parse JSON chunk
                    if let Ok(chunk_data) = serde_json::from_str::<serde_json::Value>(json_str) {
                        if let Some(choices) = chunk_data.get("choices").and_then(|c| c.as_array()) {
                            if let Some(choice) = choices.first() {
                                if let Some(delta) = choice.get("delta") {
                                    // Handle text content
                                    if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                                        full_content.push_str(content);
                                        on_chunk(content);
                                    }

                                    // Handle tool calls - accumulate incrementally
                                    if let Some(tc_array) = delta.get("tool_calls").and_then(|t| t.as_array()) {
                                        for tc_delta in tc_array {
                                            if let Some(index) = tc_delta.get("index").and_then(|i| i.as_u64()) {
                                                let index = index as usize;
                                                let entry = tool_calls_accumulator.entry(index).or_insert((String::new(), String::new(), String::new()));

                                                // Accumulate id, name, and arguments
                                                if let Some(id) = tc_delta.get("id").and_then(|i| i.as_str()) {
                                                    entry.0 = id.to_string();
                                                }
                                                if let Some(func) = tc_delta.get("function") {
                                                    if let Some(name) = func.get("name").and_then(|n| n.as_str()) {
                                                        entry.1 = name.to_string();
                                                    }
                                                    if let Some(args) = func.get("arguments").and_then(|a| a.as_str()) {
                                                        entry.2.push_str(args);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // Check finish_reason
                                if let Some(finish_reason) = choice.get("finish_reason").and_then(|f| f.as_str()) {
                                    tracing::info!("Finish reason: {}", finish_reason);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Build final tool_calls from accumulator
        let tool_calls = if !tool_calls_accumulator.is_empty() {
            let mut calls = Vec::new();
            for (_, (id, name, arguments)) in tool_calls_accumulator {
                tracing::info!("Assembled tool call: {} with args: {}", name, arguments);
                calls.push(ToolCall {
                    id,
                    tool_type: "function".to_string(),
                    function: crate::berrycode::tools::FunctionCall {
                        name,
                        arguments,
                    },
                });
            }
            Some(calls)
        } else {
            None
        };

        // Determine response type
        let response = if let Some(tc) = tool_calls {
            LLMResponse::ToolCalls(tc)
        } else if !full_content.is_empty() {
            LLMResponse::Text(full_content)
        } else {
            return Err(anyhow!("Response has neither content nor tool_calls"));
        };

        Ok((response, 0, 0))  // Token counts not available in streaming mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_provider() {
        assert!(matches!(
            LLMClient::detect_provider("gpt-4"),
            LLMProvider::OpenAI
        ));
        assert!(matches!(
            LLMClient::detect_provider("claude-3-opus"),
            LLMProvider::Anthropic
        ));
    }
}
