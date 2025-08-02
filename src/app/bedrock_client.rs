use crate::log_debug;
use aws_config::BehaviorVersion;
use aws_sdk_bedrock::Client as BedrockClient;
use aws_sdk_bedrockruntime::operation::converse::ConverseError;
use aws_sdk_bedrockruntime::types::{ContentBlock, ConversationRole, Message};
use aws_sdk_bedrockruntime::Client as BedrockRuntimeClient;
use aws_types::region::Region;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::error::Error;
use tokio::runtime::Runtime;
use tracing::{error, info};

#[derive(Debug)]
struct BedrockConverseError(String);
impl std::fmt::Display for BedrockConverseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Can't invoke. Reason {}", self.0)
    }
}
impl std::error::Error for BedrockConverseError {}
impl From<&str> for BedrockConverseError {
    fn from(value: &str) -> Self {
        BedrockConverseError(value.to_string())
    }
}
impl From<&ConverseError> for BedrockConverseError {
    fn from(value: &ConverseError) -> Self {
        BedrockConverseError::from(match value {
            ConverseError::ModelTimeoutException(_) => "Model took too long",
            ConverseError::ModelNotReadyException(_) => "Model is not ready",
            _ => "Unknown",
        })
    }
}

/// Represents a foundation model provider
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelProvider {
    pub name: String,
    pub display_name: String,
    pub provider_id: String,
}

/// Represents a foundation model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FoundationModel {
    pub model_id: String,
    pub model_name: String,
    pub provider_name: String,
    pub model_arn: String,
    pub input_modalities: Vec<String>,
    pub output_modalities: Vec<String>,
    pub supported_inference_types: Vec<String>,
}

/// Chat message for converse API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Bedrock API client for the chat interface
#[derive(Debug)]
pub struct BedrockApiClient {
    pub region: String,
    pub available_models: Vec<FoundationModel>,
    pub model_providers: Vec<ModelProvider>,
    bedrock_client: Option<BedrockClient>,
    bedrock_runtime_client: Option<BedrockRuntimeClient>,
    last_token_refresh: Option<chrono::DateTime<Utc>>,
}

impl Default for BedrockApiClient {
    fn default() -> Self {
        Self {
            region: "us-east-1".to_string(), // Default to US East 1
            available_models: Vec::new(),
            model_providers: Vec::new(),
            bedrock_client: None,
            bedrock_runtime_client: None,
            last_token_refresh: None,
        }
    }
}

impl BedrockApiClient {
    /// Create a new Bedrock API client
    pub fn new(region: String) -> Self {
        Self {
            region,
            ..Default::default()
        }
    }

    /// Initialize the Bedrock client with AWS credentials
    pub fn initialize_with_credentials(
        &mut self,
        credentials: aws_sdk_bedrock::config::Credentials,
    ) -> Result<(), Box<dyn Error>> {
        info!(
            "Initializing Bedrock client with credentials for region: {}",
            self.region
        );

        // Debug: Log partial credential info for troubleshooting (without exposing secrets)
        info!(
            "Using AWS credentials - access key prefix: {}...",
            if credentials.access_key_id().len() > 4 {
                &credentials.access_key_id()
            } else {
                "INVALID"
            }
        );

        info!(
            "Session token present: {}",
            credentials.session_token().is_some()
        );

        // Create a new Tokio runtime
        let runtime = match Runtime::new() {
            Ok(rt) => {
                log_debug!("Successfully created Tokio runtime");
                rt
            }
            Err(e) => {
                error!("Failed to create Tokio runtime: {}", e);
                return Err(Box::new(e));
            }
        };

        runtime.block_on(async {
            // Set the region for Bedrock
            let region = Region::new(self.region.clone());
            log_debug!("Setting AWS region: {}", self.region);

            // Create a new config with credentials
            log_debug!("Building AWS SDK config with credentials and region for detailed logging");

            // Create the main config
            let config = aws_config::defaults(BehaviorVersion::latest())
                .region(region)
                .credentials_provider(credentials)
                .load()
                .await;

            // Create bedrock management client
            log_debug!("Creating Bedrock management client");
            self.bedrock_client = Some(BedrockClient::new(&config));

            // Create Bedrock runtime client
            log_debug!("Creating Bedrock runtime client for model invocation");
            self.bedrock_runtime_client = Some(BedrockRuntimeClient::new(&config));

            // Record the refresh time
            self.last_token_refresh = Some(Utc::now());
            info!(
                "Bedrock clients initialized successfully at {}",
                self.last_token_refresh.unwrap()
            );
        });

        // Verify the clients were created successfully
        if !self.is_initialized() {
            error!("Failed to initialize one or both Bedrock clients");
            return Err("Failed to initialize Bedrock clients".into());
        }

        info!("Bedrock API client initialization complete");
        Ok(())
    }

    /// List available foundation models by querying AWS Bedrock
    pub fn list_foundation_models(&mut self) -> Result<Vec<FoundationModel>, Box<dyn Error>> {
        info!("Querying AWS Bedrock API for available foundation models");

        if self.bedrock_client.is_none() {
            return Err("Bedrock client not initialized. Please authenticate first.".into());
        }

        // Create a new Tokio runtime for the async call
        let runtime = match Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                error!("Failed to create Tokio runtime: {}", e);
                return Err(Box::new(e));
            }
        };

        // Fetch models from AWS Bedrock
        let models = runtime.block_on(async {
            let client = self.bedrock_client.as_ref().unwrap();

            // Call the Bedrock API to list foundation models
            let resp = match client.list_foundation_models().send().await {
                Ok(response) => response,
                Err(e) => {
                    error!("Failed to query Bedrock API for models: {}", e);
                    return Err(format!("Failed to query Bedrock API: {}", e));
                }
            };

            // Get the list of models from the response
            let aws_models = match resp.model_summaries {
                Some(models) => models,
                None => {
                    info!("No models returned from Bedrock API");
                    return Ok(Vec::new());
                }
            };

            // Convert AWS models to our FoundationModel format
            let mut result_models = Vec::new();

            // Process each model returned from AWS
            for model in aws_models {
                // Extract model ID (modelId field) - in newer AWS SDK, this is a String not Option<String>
                let model_id = if model.model_id.is_empty() {
                    continue; // Skip models without ID
                } else {
                    model.model_id.clone()
                };

                // Extract ARN - in newer AWS SDK, this is a String not Option<String>
                let model_arn = if model.model_arn.is_empty() {
                    continue; // Skip models without ARN
                } else {
                    model.model_arn.clone()
                };

                // Skip models that aren't text models or don't support Converse API
                // Only include models from providers known to work with Converse API
                let provider_name = if model_id.starts_with("anthropic.") {
                    "Anthropic".to_string()
                } else if model_id.starts_with("amazon.") {
                    "Amazon".to_string()
                } else if model_id.starts_with("meta.") {
                    "Meta".to_string()
                } else if model_id.starts_with("deepseek.") {
                    "DeepSeek".to_string()
                } else if model_id.starts_with("mistral.") {
                    "Mistral AI".to_string()
                } else if model_id.starts_with("cohere.") {
                    "Cohere".to_string()
                } else if model_id.starts_with("ai21.") {
                    "AI21 Labs".to_string()
                } else {
                    continue; // Skip other providers
                };

                // Check if the model supports Converse API
                if !is_converse_compatible(&model_id) {
                    continue; // Skip models that don't support Converse API
                };

                // Format a friendly name from the model ID
                let model_name = if model_id.starts_with("anthropic.claude-") {
                    // "anthropic.claude-3-haiku-20240307-v1:0" -> "Claude 3 Haiku"
                    let parts: Vec<&str> = model_id
                        .split('.')
                        .nth(1)
                        .unwrap_or("")
                        .split('-')
                        .collect();

                    if parts.len() >= 3 {
                        let version = parts[1].to_string(); // "3" or "3-5" or "3-7"
                        let model_type = parts[2].to_string(); // "haiku", "sonnet", "opus"

                        // Capitalize the first letter of model_type
                        let model_type = model_type
                            .chars()
                            .next()
                            .unwrap_or('_')
                            .to_uppercase()
                            .to_string()
                            + &model_type[1..];

                        format!("Claude {} {}", version, model_type)
                    } else {
                        format!("Claude ({})", model_id)
                    }
                } else if model_id.starts_with("amazon.nova-") {
                    // amazon.nova-pro-v1:0 -> "Amazon Nova Pro"
                    let parts: Vec<&str> = model_id
                        .split('.')
                        .nth(1)
                        .unwrap_or("")
                        .split('-')
                        .collect();

                    if parts.len() >= 2 {
                        let model_type = parts[1].to_string(); // "pro", "lite", etc.

                        // Capitalize the first letter of model_type
                        let model_type = model_type
                            .chars()
                            .next()
                            .unwrap_or('_')
                            .to_uppercase()
                            .to_string()
                            + &model_type[1..];

                        format!("Amazon Nova {}", model_type)
                    } else {
                        format!("Amazon Nova ({})", model_id)
                    }
                } else if model_id.starts_with("amazon.bedrock-nova-") {
                    // amazon.bedrock-nova-2023-05-12:0 -> "Amazon Bedrock Nova"
                    let parts: Vec<&str> = model_id
                        .split('.')
                        .nth(1)
                        .unwrap_or("")
                        .split('-')
                        .collect();

                    if parts.len() >= 2 {
                        let model_type = parts[1].to_string(); // "nova"

                        // Capitalize the first letter of model_type
                        let model_type = model_type
                            .chars()
                            .next()
                            .unwrap_or('_')
                            .to_uppercase()
                            .to_string()
                            + &model_type[1..];

                        format!("Amazon {}", model_type)
                    } else {
                        format!("Amazon Nova ({})", model_id)
                    }
                } else if model_id.starts_with("meta.llama") {
                    // meta.llama3-70b-instruct-v1:0 -> "Llama 3 70B Instruct"
                    let parts: Vec<&str> = model_id
                        .split('.')
                        .nth(1)
                        .unwrap_or("")
                        .split('-')
                        .collect();

                    if parts.len() >= 3 {
                        let version = if parts[0].starts_with("llama") {
                            parts[0][5..].to_string() // Extract number after "llama"
                        } else {
                            "".to_string()
                        };

                        let size = parts[1].to_string(); // "70b"
                        let model_type = if parts.len() > 2 {
                            parts[2].to_string()
                        } else {
                            "".to_string()
                        }; // "instruct"

                        // Capitalize model_type
                        let model_type = if !model_type.is_empty() {
                            model_type
                                .chars()
                                .next()
                                .unwrap_or('_')
                                .to_uppercase()
                                .to_string()
                                + &model_type[1..]
                        } else {
                            "".to_string()
                        };

                        if !model_type.is_empty() {
                            format!("Llama {} {} {}", version, size.to_uppercase(), model_type)
                        } else {
                            format!("Llama {} {}", version, size.to_uppercase())
                        }
                    } else {
                        format!("Llama ({})", model_id)
                    }
                } else if model_id.starts_with("deepseek.") {
                    // Format DeepSeek model names
                    let parts: Vec<&str> = model_id
                        .split('.')
                        .nth(1)
                        .unwrap_or("")
                        .split('-')
                        .collect();
                    format!("DeepSeek {}", parts.join(" "))
                } else {
                    // Generic case - just use the model ID as name
                    model_id.clone()
                };

                // Extract and convert modalities to strings
                let input_modalities = if let Some(modalities) = &model.input_modalities {
                    // Convert ModelModality to String
                    modalities.iter().map(|m| format!("{:?}", m)).collect()
                } else {
                    vec!["TEXT".to_string()] // Assume TEXT if not specified
                };

                let output_modalities = if let Some(modalities) = &model.output_modalities {
                    // Convert ModelModality to String
                    modalities.iter().map(|m| format!("{:?}", m)).collect()
                } else {
                    vec!["TEXT".to_string()] // Assume TEXT if not specified
                };

                // Extract supported inference types - using a fixed value as inference_types might not exist
                let inference_types = vec!["ON_DEMAND".to_string()]; // Default to ON_DEMAND

                // Create our FoundationModel structure
                let foundation_model = FoundationModel {
                    model_id: if model_id.ends_with(":0") {
                        model_id.clone() // Don't add suffix if it already has one
                    } else {
                        format!("{}:0", model_id) // Add ':0' suffix for AWS API compatibility
                    },
                    model_name,
                    provider_name,
                    model_arn,
                    input_modalities,
                    output_modalities,
                    supported_inference_types: inference_types,
                };

                result_models.push(foundation_model);
            }

            // Sort models: first by provider, then by name
            result_models.sort_by(|a, b| {
                if a.provider_name == b.provider_name {
                    a.model_name.cmp(&b.model_name)
                } else {
                    a.provider_name.cmp(&b.provider_name)
                }
            });

            Ok(result_models)
        })?;

        // Log available model IDs for debugging
        let model_ids: Vec<String> = models
            .iter()
            .map(|m| format!("{} ({})", m.model_name, m.model_id))
            .collect();
        info!(
            "Available models in Bedrock client: {}",
            model_ids.join(", ")
        );

        // Update the available models
        self.available_models = models.clone();

        // Extract and organize model providers
        let mut providers = std::collections::HashMap::new();
        for model in &self.available_models {
            providers
                .entry(model.provider_name.clone())
                .or_insert_with(|| ModelProvider {
                    name: model.provider_name.clone(),
                    display_name: format_provider_name(&model.provider_name),
                    provider_id: model.provider_name.to_lowercase().replace(" ", "_"),
                });
        }

        // Update providers list
        self.model_providers = providers.values().cloned().collect();
        self.model_providers
            .sort_by(|a, b| a.display_name.cmp(&b.display_name));

        // Add fallback models if no models were returned (e.g., due to permissions)
        if self.available_models.is_empty() {
            info!("No models found from Bedrock API. Adding fallback models for compatibility.");

            // Add Nova Pro as primary fallback model (default)
            self.available_models.push(FoundationModel {
                model_id: "amazon.nova-pro-v1:0".to_string(), // Include :0 suffix for compatibility
                model_name: "Amazon Nova Pro (Fallback)".to_string(),
                provider_name: "Amazon".to_string(),
                model_arn: "arn:aws:bedrock:us-east-1::foundation-model/amazon.nova-pro-v1"
                    .to_string(),
                input_modalities: vec!["TEXT".to_string()],
                output_modalities: vec!["TEXT".to_string()],
                supported_inference_types: vec!["ON_DEMAND".to_string()],
            });

            // Add provider if not already present
            if !self.model_providers.iter().any(|p| p.name == "Amazon") {
                self.model_providers.push(ModelProvider {
                    name: "Amazon".to_string(),
                    display_name: "Amazon".to_string(),
                    provider_id: "amazon".to_string(),
                });
            }

            // Add Claude 3 Haiku as a secondary fallback model
            self.available_models.push(FoundationModel {
                model_id: "anthropic.claude-3-haiku-20240307-v1:0".to_string(), // Include :0 suffix for compatibility
                model_name: "Claude 3 Haiku (Fallback)".to_string(),
                provider_name: "Anthropic".to_string(),
                model_arn: "arn:aws:bedrock:us-east-1::foundation-model/anthropic.claude-3-haiku-20240307-v1".to_string(),
                input_modalities: vec!["TEXT".to_string()],
                output_modalities: vec!["TEXT".to_string()],
                supported_inference_types: vec!["ON_DEMAND".to_string()],
            });

            // Add provider if not already present
            if !self.model_providers.iter().any(|p| p.name == "Anthropic") {
                self.model_providers.push(ModelProvider {
                    name: "Anthropic".to_string(),
                    display_name: "Anthropic".to_string(),
                    provider_id: "anthropic".to_string(),
                });
            }
        }

        Ok(self.available_models.clone())
    }

    /// Get the model ID for a model name
    pub fn get_model_id_for_name(&self, model_name: &str) -> Option<String> {
        self.available_models
            .iter()
            .find(|model| model.model_name == model_name)
            .map(|model| model.model_id.clone())
    }

    /// Invoke the model using the Converse API for chat
    pub fn converse_with_model(
        &self,
        model_id: &str,
        messages: &[ChatMessage],
    ) -> Result<String, Box<dyn Error>> {
        if self.bedrock_runtime_client.is_none() {
            error!(
                "Bedrock runtime client not initialized for model_id: {}",
                model_id
            );
            return Err("Bedrock runtime client not initialized".into());
        }

        info!("Starting Bedrock API call for model_id: {}", model_id);
        info!("Sending {} messages to Bedrock API", messages.len());

        // Debugging: Log all message roles
        let msg_roles: Vec<&str> = messages.iter().map(|m| m.role.as_str()).collect();
        info!("Message roles being sent: {:?}", msg_roles);

        // Create a new Tokio runtime
        let runtime = Runtime::new().map_err(|e| {
            error!("Failed to create Tokio runtime: {}", e);
            Box::new(e) as Box<dyn Error>
        })?;

        // Use the Converse API with all messages in one request
        let response = runtime.block_on(async {
            let client = self.bedrock_runtime_client.as_ref().unwrap();

            // Convert each user message to the AWS SDK format
            let mut aws_messages = Vec::new();

            for (i, msg) in messages.iter().enumerate() {
                // Map role strings to ConversationRole enum
                let role = match msg.role.to_lowercase().as_str() {
                    "user" => ConversationRole::User,
                    "assistant" => ConversationRole::Assistant,
                    // System roles to user for compatibility
                    _ => {
                        if msg.role.to_lowercase() == "system" {
                            info!("Converting system role to user role for compatibility");
                        } else {
                            error!("Unknown role '{}', defaulting to 'user'", msg.role);
                        }
                        ConversationRole::User
                    }
                };

                // Store role for logging
                let role_str = format!("{:?}", role);

                // Create message with text content
                let message = Message::builder()
                    .role(role)
                    .content(ContentBlock::Text(msg.content.clone()))
                    .build()
                    .map_err(|e| {
                        error!("Failed to build message {}: {:?}", i, e);
                        Box::<dyn Error>::from("Failed to build message for Converse API")
                    })?;

                // Add to message vector
                aws_messages.push(message);
                info!("Prepared message {} with role {}", i, role_str);
            }

            info!("Sending conversation with {} messages", aws_messages.len());

            // The AWS Bedrock API supports sending multiple messages in a conversation
            // We'll use the set_messages method to send all conversation messages at once
            if aws_messages.is_empty() {
                return Err(Box::<dyn Error>::from("No messages to send"));
            }

            // We'll send all messages to preserve the full conversation context

            // Create the Converse request with the message
            let request = client
                .converse()
                .model_id(model_id)
                .set_messages(Some(aws_messages)); // Pass all messages using set_messages

            // Log detailed request information for debugging
            info!("Converse API request details: {:?}", request);

            let converse_result = request.send().await.map_err(|e| {
                error!("Error during Bedrock Converse API call: {}", e);
                error!(
                    "API call details: model_id={}, region={}",
                    model_id, self.region
                );

                // Log the raw response error details
                info!("API Error detailed string representation: {:#?}", e);

                // Try to extract more error details
                match &e {
                    aws_sdk_bedrockruntime::error::SdkError::ServiceError(ctx) => {
                        error!("Service error: {:?}", ctx.err());
                        // ctx.raw() returns a reference directly, not an Option
                        let raw_response = ctx.raw();
                        info!("Raw HTTP response status: {}", raw_response.status());
                        info!("Raw HTTP response headers: {:#?}", raw_response.headers());

                        // Try to get the response body
                        if let Ok(body) =
                            std::str::from_utf8(raw_response.body().bytes().unwrap_or_default())
                        {
                            if !body.is_empty() {
                                info!("Raw HTTP response body: {}", body);
                            }
                        }
                    }
                    aws_sdk_bedrockruntime::error::SdkError::ResponseError(err) => {
                        error!("Response error: {:?}", err);
                    }
                    aws_sdk_bedrockruntime::error::SdkError::TimeoutError(err) => {
                        error!("Timeout error: {:?}", err);
                    }
                    aws_sdk_bedrockruntime::error::SdkError::DispatchFailure(err) => {
                        error!("Dispatch failure: {:?}", err);
                    }
                    _ => {
                        error!("Other error type: {:?}", e);
                    }
                }

                Box::new(e) as Box<dyn Error>
            })?;

            // Extract the response content
            info!("Extracting response from API result");
            if let Some(output) = converse_result.output() {
                if let Ok(message) = output.as_message() {
                    if let Some(content) = message.content().first() {
                        if let Ok(text) = content.as_text() {
                            info!("Successfully extracted text from Converse API response");
                            return Ok::<String, Box<dyn Error>>(text.to_string());
                        }
                    }
                }
            }

            // If extraction fails, try to get more information
            error!("Could not extract text from Converse API response in expected format");
            let output_debug = format!("{:?}", converse_result);
            info!("Raw Converse API response: {}", output_debug);

            Ok::<String, Box<dyn Error>>(
                "Could not parse model response. Please check logs for details.".to_string(),
            )
        })?;

        info!(
            "Bedrock API call completed successfully, response length: {}",
            response.len()
        );
        Ok(response)
    }

    /// Check if the client is initialized
    pub fn is_initialized(&self) -> bool {
        self.bedrock_client.is_some() && self.bedrock_runtime_client.is_some()
    }

    /// Run a diagnostic test on the Converse API with a single test message
    pub fn test_converse_api(&self, model_id: &str) -> Result<(), Box<dyn Error>> {
        if self.bedrock_runtime_client.is_none() {
            return Err("Bedrock runtime client not initialized".into());
        }

        info!(
            "Running diagnostic test on Converse API with model_id: {}",
            model_id
        );

        // Create a new Tokio runtime
        let runtime = Runtime::new().map_err(|e| {
            error!("Failed to create Tokio runtime: {}", e);
            Box::new(e) as Box<dyn Error>
        })?;

        runtime.block_on(async {
            let client = self.bedrock_runtime_client.as_ref().unwrap();

            // Create a simple test message
            let test_message = Message::builder()
                .role(ConversationRole::User)
                .content(ContentBlock::Text("Hello".to_string()))
                .build()
                .map_err(|e| {
                    error!("Failed to build test message: {:?}", e);
                    Box::<dyn Error>::from("Failed to build test message")
                })?;

            // Create a vector of messages for the test
            let messages = vec![test_message];

            // Send all messages using set_messages
            let request = client
                .converse()
                .model_id(model_id)
                .set_messages(Some(messages));
            info!("Converse API request structure: {:?}", request);

            // Send the test message
            info!("Sending test request to Bedrock Converse API...");
            let result = request.send().await.map_err(|e| {
                error!("Error during Converse API test: {}", e);
                error!(
                    "API call details: model_id={}, region={}",
                    model_id, self.region
                );

                // Log the raw response error details
                info!("API Error detailed string representation: {:#?}", e);

                // Extract more error details if available
                match &e {
                    aws_sdk_bedrockruntime::error::SdkError::ServiceError(ctx) => {
                        error!("Service error: {:?}", ctx.err());
                        // ctx.raw() returns a reference directly, not an Option
                        let raw_response = ctx.raw();
                        info!("Raw HTTP response status: {}", raw_response.status());
                        info!("Raw HTTP response headers: {:#?}", raw_response.headers());

                        // Try to get the response body
                        if let Ok(body) =
                            std::str::from_utf8(raw_response.body().bytes().unwrap_or_default())
                        {
                            if !body.is_empty() {
                                info!("Raw HTTP response body: {}", body);
                            }
                        }
                    }
                    aws_sdk_bedrockruntime::error::SdkError::ResponseError(err) => {
                        error!("Response error: {:?}", err);
                    }
                    aws_sdk_bedrockruntime::error::SdkError::TimeoutError(err) => {
                        error!("Timeout error: {:?}", err);
                    }
                    aws_sdk_bedrockruntime::error::SdkError::DispatchFailure(err) => {
                        error!("Dispatch failure: {:?}", err);
                    }
                    _ => {
                        error!("Other error type: {:?}", e);
                    }
                }

                Box::<dyn Error>::from(format!("Converse API test failed: {}", e))
            })?;

            // Log the successful response
            info!("✅ Converse API test successful!");
            info!("Response summary: {:?}", result);

            Ok(())
        })
    }
}

/// Format provider name for display
fn format_provider_name(provider: &str) -> String {
    match provider {
        "Amazon" => "Amazon",
        "Anthropic" => "Anthropic",
        "Cohere" => "Cohere",
        "Meta" => "Meta",
        "Mistral AI" => "Mistral AI",
        "AI21 Labs" => "AI21 Labs",
        _ => provider,
    }
    .to_string()
}

/// Check if a model is compatible with the Converse API
fn is_converse_compatible(model_id: &str) -> bool {
    // All of these model prefixes are documented as compatible with Converse API
    if model_id.starts_with("anthropic.claude-") {
        return true; // All Claude models support Converse
    }

    if model_id.starts_with("amazon.nova-") {
        return true; // Amazon Nova models support Converse
    }

    if model_id.starts_with("meta.llama") {
        return true; // All Meta Llama models support Converse
    }

    if model_id.starts_with("mistral.") {
        return true; // All Mistral AI models support Converse
    }

    if model_id.starts_with("cohere.command") {
        return true; // Cohere Command models support Converse
    }

    if model_id.starts_with("ai21.jamba") {
        return true; // AI21 Jamba models support Converse
    }

    if model_id.starts_with("deepseek.") {
        return true; // DeepSeek models support Converse
    }

    // Default to false for any other models
    false
}
