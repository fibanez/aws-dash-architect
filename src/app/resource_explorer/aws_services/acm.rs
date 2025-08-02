use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_acm as acm;
use std::sync::Arc;

pub struct AcmService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl AcmService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List ACM Certificates
    pub async fn list_certificates(
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

        let client = acm::Client::new(&aws_config);
        let mut paginator = client.list_certificates().into_paginator().send();

        let mut certificates = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(certificate_summary_list) = page.certificate_summary_list {
                for cert in certificate_summary_list {
                    // Get detailed certificate information
                    if let Some(cert_arn) = &cert.certificate_arn {
                        if let Ok(cert_details) =
                            self.describe_certificate_internal(&client, cert_arn).await
                        {
                            certificates.push(cert_details);
                        } else {
                            // Fallback to basic certificate info if describe fails
                            let cert_json = self.certificate_summary_to_json(&cert);
                            certificates.push(cert_json);
                        }
                    } else {
                        // Fallback to basic certificate info if no ARN
                        let cert_json = self.certificate_summary_to_json(&cert);
                        certificates.push(cert_json);
                    }
                }
            }
        }

        Ok(certificates)
    }

    /// Get detailed information for specific ACM certificate
    pub async fn describe_certificate(
        &self,
        account_id: &str,
        region: &str,
        certificate_arn: &str,
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

        let client = acm::Client::new(&aws_config);
        self.describe_certificate_internal(&client, certificate_arn)
            .await
    }

    async fn describe_certificate_internal(
        &self,
        client: &acm::Client,
        certificate_arn: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .describe_certificate()
            .certificate_arn(certificate_arn)
            .send()
            .await?;

        if let Some(certificate) = response.certificate {
            Ok(self.certificate_detail_to_json(&certificate))
        } else {
            Err(anyhow::anyhow!("Certificate {} not found", certificate_arn))
        }
    }

    fn certificate_summary_to_json(
        &self,
        cert: &acm::types::CertificateSummary,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(cert_arn) = &cert.certificate_arn {
            json.insert(
                "CertificateArn".to_string(),
                serde_json::Value::String(cert_arn.clone()),
            );
        }

        if let Some(domain_name) = &cert.domain_name {
            json.insert(
                "DomainName".to_string(),
                serde_json::Value::String(domain_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(domain_name.clone()),
            );
        }

        if let Some(status) = &cert.status {
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(status.as_str().to_string()),
            );
        }

        if let Some(key_algorithm) = &cert.key_algorithm {
            json.insert(
                "KeyAlgorithm".to_string(),
                serde_json::Value::String(key_algorithm.as_str().to_string()),
            );
        }

        if let Some(key_usages) = &cert.key_usages {
            let usages: Vec<String> = key_usages
                .iter()
                .map(|usage| format!("{:?}", usage))
                .collect();
            json.insert(
                "KeyUsages".to_string(),
                serde_json::Value::Array(
                    usages.into_iter().map(serde_json::Value::String).collect(),
                ),
            );
        }

        if let Some(extended_key_usages) = &cert.extended_key_usages {
            let ext_usages: Vec<String> = extended_key_usages
                .iter()
                .map(|usage| format!("{:?}", usage))
                .collect();
            json.insert(
                "ExtendedKeyUsages".to_string(),
                serde_json::Value::Array(
                    ext_usages
                        .into_iter()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            );
        }

        if let Some(created_at) = cert.created_at {
            json.insert(
                "CreatedAt".to_string(),
                serde_json::Value::String(created_at.to_string()),
            );
        }

        if let Some(issued_at) = cert.issued_at {
            json.insert(
                "IssuedAt".to_string(),
                serde_json::Value::String(issued_at.to_string()),
            );
        }

        if let Some(not_before) = cert.not_before {
            json.insert(
                "NotBefore".to_string(),
                serde_json::Value::String(not_before.to_string()),
            );
        }

        if let Some(not_after) = cert.not_after {
            json.insert(
                "NotAfter".to_string(),
                serde_json::Value::String(not_after.to_string()),
            );
        }

        serde_json::Value::Object(json)
    }

    fn certificate_detail_to_json(
        &self,
        certificate: &acm::types::CertificateDetail,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(cert_arn) = &certificate.certificate_arn {
            json.insert(
                "CertificateArn".to_string(),
                serde_json::Value::String(cert_arn.clone()),
            );
        }

        if let Some(domain_name) = &certificate.domain_name {
            json.insert(
                "DomainName".to_string(),
                serde_json::Value::String(domain_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(domain_name.clone()),
            );
        }

        if let Some(subject_alternative_names) = &certificate.subject_alternative_names {
            json.insert(
                "SubjectAlternativeNames".to_string(),
                serde_json::Value::Array(
                    subject_alternative_names
                        .iter()
                        .map(|s| serde_json::Value::String(s.clone()))
                        .collect(),
                ),
            );
        }

        if let Some(domain_validation_options) = &certificate.domain_validation_options {
            let validation_options: Vec<serde_json::Value> = domain_validation_options
                .iter()
                .map(|option| {
                    let mut option_json = serde_json::Map::new();
                    if !option.domain_name.is_empty() {
                        option_json.insert(
                            "DomainName".to_string(),
                            serde_json::Value::String(option.domain_name.clone()),
                        );
                    }
                    if let Some(validation_domain) = &option.validation_domain {
                        option_json.insert(
                            "ValidationDomain".to_string(),
                            serde_json::Value::String(validation_domain.clone()),
                        );
                    }
                    if let Some(validation_status) = &option.validation_status {
                        option_json.insert(
                            "ValidationStatus".to_string(),
                            serde_json::Value::String(format!("{:?}", validation_status)),
                        );
                    }
                    if let Some(validation_method) = &option.validation_method {
                        option_json.insert(
                            "ValidationMethod".to_string(),
                            serde_json::Value::String(format!("{:?}", validation_method)),
                        );
                    }
                    serde_json::Value::Object(option_json)
                })
                .collect();
            json.insert(
                "DomainValidationOptions".to_string(),
                serde_json::Value::Array(validation_options),
            );
        }

        if let Some(subject) = &certificate.subject {
            json.insert(
                "Subject".to_string(),
                serde_json::Value::String(subject.clone()),
            );
        }

        if let Some(issuer) = &certificate.issuer {
            json.insert(
                "Issuer".to_string(),
                serde_json::Value::String(issuer.clone()),
            );
        }

        if let Some(created_at) = certificate.created_at {
            json.insert(
                "CreatedAt".to_string(),
                serde_json::Value::String(created_at.to_string()),
            );
        }

        if let Some(issued_at) = certificate.issued_at {
            json.insert(
                "IssuedAt".to_string(),
                serde_json::Value::String(issued_at.to_string()),
            );
        }

        if let Some(status) = &certificate.status {
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(status.as_str().to_string()),
            );
        }

        if let Some(revoked_at) = certificate.revoked_at {
            json.insert(
                "RevokedAt".to_string(),
                serde_json::Value::String(revoked_at.to_string()),
            );
        }

        if let Some(revocation_reason) = &certificate.revocation_reason {
            json.insert(
                "RevocationReason".to_string(),
                serde_json::Value::String(revocation_reason.as_str().to_string()),
            );
        }

        if let Some(not_before) = certificate.not_before {
            json.insert(
                "NotBefore".to_string(),
                serde_json::Value::String(not_before.to_string()),
            );
        }

        if let Some(not_after) = certificate.not_after {
            json.insert(
                "NotAfter".to_string(),
                serde_json::Value::String(not_after.to_string()),
            );
        }

        if let Some(key_algorithm) = &certificate.key_algorithm {
            json.insert(
                "KeyAlgorithm".to_string(),
                serde_json::Value::String(key_algorithm.as_str().to_string()),
            );
        }

        if let Some(signature_algorithm) = &certificate.signature_algorithm {
            json.insert(
                "SignatureAlgorithm".to_string(),
                serde_json::Value::String(signature_algorithm.clone()),
            );
        }

        if let Some(key_usages) = &certificate.key_usages {
            let usages: Vec<String> = key_usages
                .iter()
                .map(|usage| format!("{:?}", usage))
                .collect();
            json.insert(
                "KeyUsages".to_string(),
                serde_json::Value::Array(
                    usages.into_iter().map(serde_json::Value::String).collect(),
                ),
            );
        }

        if let Some(extended_key_usages) = &certificate.extended_key_usages {
            let ext_usages: Vec<String> = extended_key_usages
                .iter()
                .map(|usage| format!("{:?}", usage))
                .collect();
            json.insert(
                "ExtendedKeyUsages".to_string(),
                serde_json::Value::Array(
                    ext_usages
                        .into_iter()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            );
        }

        // Certificate transparency logging preference is in options object
        if let Some(options) = &certificate.options {
            let mut options_json = serde_json::Map::new();
            if let Some(cert_transparency_logging_preference) =
                &options.certificate_transparency_logging_preference
            {
                options_json.insert(
                    "CertificateTransparencyLoggingPreference".to_string(),
                    serde_json::Value::String(format!(
                        "{:?}",
                        cert_transparency_logging_preference
                    )),
                );
            }
            json.insert(
                "Options".to_string(),
                serde_json::Value::Object(options_json),
            );
        }

        if let Some(renewal_eligibility) = &certificate.renewal_eligibility {
            json.insert(
                "RenewalEligibility".to_string(),
                serde_json::Value::String(format!("{:?}", renewal_eligibility)),
            );
        }

        if let Some(serial) = &certificate.serial {
            json.insert(
                "Serial".to_string(),
                serde_json::Value::String(serial.clone()),
            );
        }

        serde_json::Value::Object(json)
    }
}
