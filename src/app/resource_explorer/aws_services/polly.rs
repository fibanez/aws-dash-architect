use anyhow::{Result, Context};
use aws_sdk_polly as polly;
use std::sync::Arc;
use super::super::credentials::CredentialCoordinator;

pub struct PollyService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl PollyService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Polly voices (basic list data)
    pub async fn describe_voices(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = polly::Client::new(&aws_config);
        
        // Note: Polly describe_voices doesn't support pagination in the SDK
        let response = client
            .describe_voices()
            .send()
            .await?;

        let mut voices = Vec::new();
        if let Some(voice_list) = response.voices {
            for voice in voice_list {
                let voice_json = self.voice_to_json(&voice);
                voices.push(voice_json);
            }
        }

        Ok(voices)
    }

    /// List Polly lexicons (basic list data)
    pub async fn list_lexicons(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = polly::Client::new(&aws_config);
        
        // Use manual token pagination for list_lexicons
        let mut next_token: Option<String> = None;
        let mut lexicons = Vec::new();

        loop {
            let mut request = client.list_lexicons();
            if let Some(token) = &next_token {
                request = request.next_token(token);
            }
            
            let response = request.send().await?;
            
            if let Some(lexicon_list) = response.lexicons {
                for lexicon in lexicon_list {
                    let lexicon_json = self.lexicon_to_json(&lexicon);
                    lexicons.push(lexicon_json);
                }
            }
            
            if let Some(token) = response.next_token {
                next_token = Some(token);
            } else {
                break;
            }
        }

        Ok(lexicons)
    }

    /// List Polly speech synthesis tasks (basic list data)
    pub async fn list_speech_synthesis_tasks(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = polly::Client::new(&aws_config);
        
        let mut paginator = client
            .list_speech_synthesis_tasks()
            .into_paginator()
            .send();

        let mut tasks = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(synthesis_tasks) = page.synthesis_tasks {
                for task in synthesis_tasks {
                    let task_json = self.synthesis_task_to_json(&task);
                    tasks.push(task_json);
                }
            }
        }

        Ok(tasks)
    }

    /// Get detailed information for specific lexicon (for describe functionality)
    pub async fn get_lexicon(
        &self,
        account_id: &str,
        region: &str,
        lexicon_name: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = polly::Client::new(&aws_config);
        self.get_lexicon_internal(&client, lexicon_name).await
    }

    /// Get detailed information for specific synthesis task (for describe functionality)
    pub async fn get_speech_synthesis_task(
        &self,
        account_id: &str,
        region: &str,
        task_id: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = polly::Client::new(&aws_config);
        self.get_speech_synthesis_task_internal(&client, task_id).await
    }

    async fn get_lexicon_internal(
        &self,
        client: &polly::Client,
        lexicon_name: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .get_lexicon()
            .name(lexicon_name)
            .send()
            .await?;

        let mut json = serde_json::Map::new();
        
        json.insert("Name".to_string(), serde_json::Value::String(lexicon_name.to_string()));
        json.insert("ResourceId".to_string(), serde_json::Value::String(lexicon_name.to_string()));

        if let Some(lexicon) = response.lexicon {
            if let Some(content) = lexicon.content {
                json.insert("Content".to_string(), serde_json::Value::String(content));
            }
            if let Some(name) = lexicon.name {
                json.insert("LexiconName".to_string(), serde_json::Value::String(name));
            }
        }

        if let Some(lexicon_attributes) = response.lexicon_attributes {
            if let Some(alphabet) = lexicon_attributes.alphabet {
                json.insert("Alphabet".to_string(), serde_json::Value::String(alphabet));
            }
            if let Some(language_code) = lexicon_attributes.language_code {
                json.insert("LanguageCode".to_string(), serde_json::Value::String(language_code.as_str().to_string()));
            }
            if let Some(last_modified) = lexicon_attributes.last_modified {
                json.insert("LastModified".to_string(), serde_json::Value::String(last_modified.to_string()));
            }
            json.insert("LexemesCount".to_string(), serde_json::Value::Number(serde_json::Number::from(lexicon_attributes.lexemes_count)));
            json.insert("Size".to_string(), serde_json::Value::Number(serde_json::Number::from(lexicon_attributes.size)));
        }

        Ok(serde_json::Value::Object(json))
    }

    async fn get_speech_synthesis_task_internal(
        &self,
        client: &polly::Client,
        task_id: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .get_speech_synthesis_task()
            .task_id(task_id)
            .send()
            .await?;

        if let Some(synthesis_task) = response.synthesis_task {
            Ok(self.synthesis_task_to_json(&synthesis_task))
        } else {
            Err(anyhow::anyhow!("Speech synthesis task {} not found", task_id))
        }
    }

    fn voice_to_json(&self, voice: &polly::types::Voice) -> serde_json::Value {
        let mut json = serde_json::Map::new();
        
        if let Some(voice_id) = &voice.id {
            json.insert("VoiceId".to_string(), serde_json::Value::String(voice_id.as_str().to_string()));
            json.insert("ResourceId".to_string(), serde_json::Value::String(voice_id.as_str().to_string()));
            json.insert("Name".to_string(), serde_json::Value::String(voice_id.as_str().to_string()));
        }

        if let Some(gender) = &voice.gender {
            json.insert("Gender".to_string(), serde_json::Value::String(gender.as_str().to_string()));
        }

        if let Some(language_code) = &voice.language_code {
            json.insert("LanguageCode".to_string(), serde_json::Value::String(language_code.as_str().to_string()));
        }

        if let Some(language_name) = &voice.language_name {
            json.insert("LanguageName".to_string(), serde_json::Value::String(language_name.clone()));
        }

        if let Some(name) = &voice.name {
            json.insert("VoiceName".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(additional_language_codes) = &voice.additional_language_codes {
            if !additional_language_codes.is_empty() {
                let codes: Vec<serde_json::Value> = additional_language_codes
                    .iter()
                    .map(|code| serde_json::Value::String(code.as_str().to_string()))
                    .collect();
                json.insert("AdditionalLanguageCodes".to_string(), serde_json::Value::Array(codes));
            }
        }

        if let Some(supported_engines) = &voice.supported_engines {
            if !supported_engines.is_empty() {
                let engines: Vec<serde_json::Value> = supported_engines
                    .iter()
                    .map(|engine| serde_json::Value::String(engine.as_str().to_string()))
                    .collect();
                json.insert("SupportedEngines".to_string(), serde_json::Value::Array(engines));
            }
        }

        json.insert("Status".to_string(), serde_json::Value::String("AVAILABLE".to_string()));

        serde_json::Value::Object(json)
    }

    fn lexicon_to_json(&self, lexicon: &polly::types::LexiconDescription) -> serde_json::Value {
        let mut json = serde_json::Map::new();
        
        if let Some(name) = &lexicon.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
            json.insert("ResourceId".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(attributes) = &lexicon.attributes {
            if let Some(alphabet) = &attributes.alphabet {
                json.insert("Alphabet".to_string(), serde_json::Value::String(alphabet.clone()));
            }
            if let Some(language_code) = &attributes.language_code {
                json.insert("LanguageCode".to_string(), serde_json::Value::String(language_code.as_str().to_string()));
            }
            if let Some(last_modified) = attributes.last_modified {
                json.insert("LastModified".to_string(), serde_json::Value::String(last_modified.to_string()));
            }
            json.insert("LexemesCount".to_string(), serde_json::Value::Number(serde_json::Number::from(attributes.lexemes_count)));
            json.insert("Size".to_string(), serde_json::Value::Number(serde_json::Number::from(attributes.size)));
        }

        json.insert("Status".to_string(), serde_json::Value::String("ACTIVE".to_string()));

        serde_json::Value::Object(json)
    }

    fn synthesis_task_to_json(&self, task: &polly::types::SynthesisTask) -> serde_json::Value {
        let mut json = serde_json::Map::new();
        
        if let Some(task_id) = &task.task_id {
            json.insert("TaskId".to_string(), serde_json::Value::String(task_id.clone()));
            json.insert("ResourceId".to_string(), serde_json::Value::String(task_id.clone()));
        }

        if let Some(task_status) = &task.task_status {
            json.insert("TaskStatus".to_string(), serde_json::Value::String(task_status.as_str().to_string()));
            json.insert("Status".to_string(), serde_json::Value::String(task_status.as_str().to_string()));
        }

        if let Some(task_status_reason) = &task.task_status_reason {
            json.insert("TaskStatusReason".to_string(), serde_json::Value::String(task_status_reason.clone()));
        }

        if let Some(output_uri) = &task.output_uri {
            json.insert("OutputUri".to_string(), serde_json::Value::String(output_uri.clone()));
        }

        if let Some(creation_time) = task.creation_time {
            json.insert("CreationTime".to_string(), serde_json::Value::String(creation_time.to_string()));
        }

        json.insert("RequestCharacters".to_string(), serde_json::Value::Number(serde_json::Number::from(task.request_characters)));

        if let Some(sns_topic_arn) = &task.sns_topic_arn {
            json.insert("SnsTopicArn".to_string(), serde_json::Value::String(sns_topic_arn.clone()));
        }

        if let Some(voice_id) = &task.voice_id {
            json.insert("VoiceId".to_string(), serde_json::Value::String(voice_id.as_str().to_string()));
        }

        if let Some(output_format) = &task.output_format {
            json.insert("OutputFormat".to_string(), serde_json::Value::String(output_format.as_str().to_string()));
        }

        if let Some(sample_rate) = &task.sample_rate {
            json.insert("SampleRate".to_string(), serde_json::Value::String(sample_rate.clone()));
        }

        if let Some(speech_mark_types) = &task.speech_mark_types {
            if !speech_mark_types.is_empty() {
                let marks: Vec<serde_json::Value> = speech_mark_types
                    .iter()
                    .map(|mark| serde_json::Value::String(mark.as_str().to_string()))
                    .collect();
                json.insert("SpeechMarkTypes".to_string(), serde_json::Value::Array(marks));
            }
        }

        if let Some(text_type) = &task.text_type {
            json.insert("TextType".to_string(), serde_json::Value::String(text_type.as_str().to_string()));
        }

        if let Some(engine) = &task.engine {
            json.insert("Engine".to_string(), serde_json::Value::String(engine.as_str().to_string()));
        }

        if let Some(language_code) = &task.language_code {
            json.insert("LanguageCode".to_string(), serde_json::Value::String(language_code.as_str().to_string()));
        }

        serde_json::Value::Object(json)
    }
}