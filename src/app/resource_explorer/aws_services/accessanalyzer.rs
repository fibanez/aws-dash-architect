use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_accessanalyzer as accessanalyzer;
use std::sync::Arc;

pub struct AccessAnalyzerService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl AccessAnalyzerService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Access Analyzers
    pub async fn list_analyzers(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = accessanalyzer::Client::new(&aws_config);
        let mut analyzers = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client.list_analyzers();
            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            for analyzer in response.analyzers {
                let analyzer_json = self.analyzer_to_json(&analyzer);
                analyzers.push(analyzer_json);
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(analyzers)
    }

    /// Get detailed information for specific Access Analyzer
    pub async fn describe_analyzer(
        &self,
        account_id: &str,
        region: &str,
        analyzer_name: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = accessanalyzer::Client::new(&aws_config);
        self.describe_analyzer_internal(&client, analyzer_name)
            .await
    }

    async fn describe_analyzer_internal(
        &self,
        client: &accessanalyzer::Client,
        analyzer_name: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .get_analyzer()
            .analyzer_name(analyzer_name)
            .send()
            .await?;

        if let Some(analyzer) = response.analyzer {
            Ok(self.analyzer_detail_to_json(&analyzer))
        } else {
            Err(anyhow::anyhow!(
                "Access Analyzer {} not found",
                analyzer_name
            ))
        }
    }

    fn analyzer_to_json(
        &self,
        analyzer: &accessanalyzer::types::AnalyzerSummary,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "AnalyzerName".to_string(),
            serde_json::Value::String(analyzer.name.clone()),
        );
        json.insert(
            "ResourceId".to_string(),
            serde_json::Value::String(analyzer.name.clone()),
        );
        json.insert(
            "Name".to_string(),
            serde_json::Value::String(analyzer.name.clone()),
        );

        json.insert(
            "AnalyzerArn".to_string(),
            serde_json::Value::String(analyzer.arn.clone()),
        );

        json.insert(
            "Type".to_string(),
            serde_json::Value::String(analyzer.r#type.as_str().to_string()),
        );

        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(analyzer.created_at.to_string()),
        );

        if let Some(last_resource_analyzed) = &analyzer.last_resource_analyzed {
            json.insert(
                "LastResourceAnalyzed".to_string(),
                serde_json::Value::String(last_resource_analyzed.to_string()),
            );
        }

        if let Some(last_resource_analyzed_at) = analyzer.last_resource_analyzed_at {
            json.insert(
                "LastResourceAnalyzedAt".to_string(),
                serde_json::Value::String(last_resource_analyzed_at.to_string()),
            );
        }

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(analyzer.status.as_str().to_string()),
        );

        if let Some(status_reason) = &analyzer.status_reason {
            let mut reason_json = serde_json::Map::new();
            reason_json.insert(
                "Code".to_string(),
                serde_json::Value::String(status_reason.code.as_str().to_string()),
            );
            json.insert(
                "StatusReason".to_string(),
                serde_json::Value::Object(reason_json),
            );
        }

        if let Some(tags) = &analyzer.tags {
            let tags_json: serde_json::Map<String, serde_json::Value> = tags
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect();
            json.insert("Tags".to_string(), serde_json::Value::Object(tags_json));
        }

        serde_json::Value::Object(json)
    }

    fn analyzer_detail_to_json(
        &self,
        analyzer: &accessanalyzer::types::AnalyzerSummary,
    ) -> serde_json::Value {
        // For detailed view, we use the same conversion as the summary
        // since Access Analyzer API returns the same structure
        self.analyzer_to_json(analyzer)
    }

    /// List findings for an analyzer
    pub async fn list_findings(
        &self,
        account_id: &str,
        region: &str,
        analyzer_arn: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = accessanalyzer::Client::new(&aws_config);
        let mut findings = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client.list_findings().analyzer_arn(analyzer_arn);
            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            for finding in response.findings {
                let finding_json = self.finding_to_json(&finding);
                findings.push(finding_json);
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(findings)
    }

    fn finding_to_json(
        &self,
        finding: &accessanalyzer::types::FindingSummary,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "FindingId".to_string(),
            serde_json::Value::String(finding.id.clone()),
        );
        json.insert(
            "ResourceType".to_string(),
            serde_json::Value::String(finding.resource_type.as_str().to_string()),
        );
        json.insert(
            "ResourceOwnerAccount".to_string(),
            serde_json::Value::String(finding.resource_owner_account.clone()),
        );

        let condition_json: serde_json::Map<String, serde_json::Value> = finding
            .condition
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
            .collect();
        json.insert(
            "Condition".to_string(),
            serde_json::Value::Object(condition_json),
        );

        if let Some(action) = &finding.action {
            let action_json: Vec<serde_json::Value> = action
                .iter()
                .map(|a| serde_json::Value::String(a.clone()))
                .collect();
            json.insert("Action".to_string(), serde_json::Value::Array(action_json));
        }

        if let Some(principal) = &finding.principal {
            let principal_json: serde_json::Map<String, serde_json::Value> = principal
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect();
            json.insert(
                "Principal".to_string(),
                serde_json::Value::Object(principal_json),
            );
        }

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(finding.status.as_str().to_string()),
        );

        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(finding.created_at.to_string()),
        );
        json.insert(
            "AnalyzedAt".to_string(),
            serde_json::Value::String(finding.analyzed_at.to_string()),
        );
        json.insert(
            "UpdatedAt".to_string(),
            serde_json::Value::String(finding.updated_at.to_string()),
        );

        if let Some(is_public) = finding.is_public {
            json.insert("IsPublic".to_string(), serde_json::Value::Bool(is_public));
        }

        serde_json::Value::Object(json)
    }
}
