use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_acmpca as acmpca;
use std::sync::Arc;

pub struct AcmPcaService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl AcmPcaService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Private Certificate Authorities
    pub async fn list_certificate_authorities(
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

        let client = acmpca::Client::new(&aws_config);
        let mut paginator = client
            .list_certificate_authorities()
            .into_paginator()
            .send();

        let mut certificate_authorities = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(certificate_authorities_list) = page.certificate_authorities {
                for ca in certificate_authorities_list {
                    // Get detailed certificate authority information
                    if let Some(ca_arn) = &ca.arn {
                        if let Ok(ca_details) = self
                            .describe_certificate_authority_internal(&client, ca_arn)
                            .await
                        {
                            certificate_authorities.push(ca_details);
                        } else {
                            // Fallback to basic CA info if describe fails
                            let ca_json = self.certificate_authority_to_json(&ca);
                            certificate_authorities.push(ca_json);
                        }
                    } else {
                        // Fallback to basic CA info if no ARN
                        let ca_json = self.certificate_authority_to_json(&ca);
                        certificate_authorities.push(ca_json);
                    }
                }
            }
        }

        Ok(certificate_authorities)
    }

    /// Get detailed information for specific Private Certificate Authority
    pub async fn describe_certificate_authority(
        &self,
        account_id: &str,
        region: &str,
        certificate_authority_arn: &str,
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

        let client = acmpca::Client::new(&aws_config);
        self.describe_certificate_authority_internal(&client, certificate_authority_arn)
            .await
    }

    async fn describe_certificate_authority_internal(
        &self,
        client: &acmpca::Client,
        certificate_authority_arn: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .describe_certificate_authority()
            .certificate_authority_arn(certificate_authority_arn)
            .send()
            .await?;

        if let Some(certificate_authority) = response.certificate_authority {
            Ok(self.certificate_authority_detail_to_json(&certificate_authority))
        } else {
            Err(anyhow::anyhow!(
                "Certificate Authority {} not found",
                certificate_authority_arn
            ))
        }
    }

    fn certificate_authority_to_json(
        &self,
        ca: &acmpca::types::CertificateAuthority,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(arn) = &ca.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));

            // Extract CA ID from ARN (last part after /)
            if let Some(ca_id) = arn.split('/').next_back() {
                json.insert(
                    "CertificateAuthorityId".to_string(),
                    serde_json::Value::String(ca_id.to_string()),
                );
            }
        }

        if let Some(owner_account) = &ca.owner_account {
            json.insert(
                "OwnerAccount".to_string(),
                serde_json::Value::String(owner_account.clone()),
            );
        }

        if let Some(created_at) = ca.created_at {
            json.insert(
                "CreatedAt".to_string(),
                serde_json::Value::String(created_at.to_string()),
            );
        }

        if let Some(last_state_change_at) = ca.last_state_change_at {
            json.insert(
                "LastStateChangeAt".to_string(),
                serde_json::Value::String(last_state_change_at.to_string()),
            );
        }

        if let Some(ca_type) = &ca.r#type {
            json.insert(
                "Type".to_string(),
                serde_json::Value::String(format!("{:?}", ca_type)),
            );
        }

        if let Some(serial) = &ca.serial {
            json.insert(
                "Serial".to_string(),
                serde_json::Value::String(serial.clone()),
            );
        }

        if let Some(status) = &ca.status {
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(format!("{:?}", status)),
            );
            json.insert(
                "State".to_string(),
                serde_json::Value::String(format!("{:?}", status)),
            );
        }

        if let Some(not_before) = ca.not_before {
            json.insert(
                "NotBefore".to_string(),
                serde_json::Value::String(not_before.to_string()),
            );
        }

        if let Some(not_after) = ca.not_after {
            json.insert(
                "NotAfter".to_string(),
                serde_json::Value::String(not_after.to_string()),
            );
        }

        if let Some(failure_reason) = &ca.failure_reason {
            json.insert(
                "FailureReason".to_string(),
                serde_json::Value::String(format!("{:?}", failure_reason)),
            );
        }

        if let Some(certificate_authority_configuration) = &ca.certificate_authority_configuration {
            let mut config_json = serde_json::Map::new();

            config_json.insert(
                "KeyAlgorithm".to_string(),
                serde_json::Value::String(format!(
                    "{:?}",
                    certificate_authority_configuration.key_algorithm
                )),
            );

            config_json.insert(
                "SigningAlgorithm".to_string(),
                serde_json::Value::String(format!(
                    "{:?}",
                    certificate_authority_configuration.signing_algorithm
                )),
            );

            if let Some(subject) = &certificate_authority_configuration.subject {
                let mut subject_json = serde_json::Map::new();

                if let Some(country) = &subject.country {
                    subject_json.insert(
                        "Country".to_string(),
                        serde_json::Value::String(country.clone()),
                    );
                }
                if let Some(organization) = &subject.organization {
                    subject_json.insert(
                        "Organization".to_string(),
                        serde_json::Value::String(organization.clone()),
                    );
                }
                if let Some(organizational_unit) = &subject.organizational_unit {
                    subject_json.insert(
                        "OrganizationalUnit".to_string(),
                        serde_json::Value::String(organizational_unit.clone()),
                    );
                }
                if let Some(distinguished_name_qualifier) = &subject.distinguished_name_qualifier {
                    subject_json.insert(
                        "DistinguishedNameQualifier".to_string(),
                        serde_json::Value::String(distinguished_name_qualifier.clone()),
                    );
                }
                if let Some(state) = &subject.state {
                    subject_json.insert(
                        "State".to_string(),
                        serde_json::Value::String(state.clone()),
                    );
                }
                if let Some(common_name) = &subject.common_name {
                    subject_json.insert(
                        "CommonName".to_string(),
                        serde_json::Value::String(common_name.clone()),
                    );
                    json.insert(
                        "Name".to_string(),
                        serde_json::Value::String(common_name.clone()),
                    ); // For display name
                }
                if let Some(serial_number) = &subject.serial_number {
                    subject_json.insert(
                        "SerialNumber".to_string(),
                        serde_json::Value::String(serial_number.clone()),
                    );
                }
                if let Some(locality) = &subject.locality {
                    subject_json.insert(
                        "Locality".to_string(),
                        serde_json::Value::String(locality.clone()),
                    );
                }
                if let Some(title) = &subject.title {
                    subject_json.insert(
                        "Title".to_string(),
                        serde_json::Value::String(title.clone()),
                    );
                }
                if let Some(surname) = &subject.surname {
                    subject_json.insert(
                        "Surname".to_string(),
                        serde_json::Value::String(surname.clone()),
                    );
                }
                if let Some(given_name) = &subject.given_name {
                    subject_json.insert(
                        "GivenName".to_string(),
                        serde_json::Value::String(given_name.clone()),
                    );
                }
                if let Some(initials) = &subject.initials {
                    subject_json.insert(
                        "Initials".to_string(),
                        serde_json::Value::String(initials.clone()),
                    );
                }
                if let Some(pseudonym) = &subject.pseudonym {
                    subject_json.insert(
                        "Pseudonym".to_string(),
                        serde_json::Value::String(pseudonym.clone()),
                    );
                }
                if let Some(generation_qualifier) = &subject.generation_qualifier {
                    subject_json.insert(
                        "GenerationQualifier".to_string(),
                        serde_json::Value::String(generation_qualifier.clone()),
                    );
                }
                if let Some(custom_attributes) = &subject.custom_attributes {
                    let custom_attrs: Vec<serde_json::Value> = custom_attributes
                        .iter()
                        .map(|attr| {
                            let mut attr_json = serde_json::Map::new();
                            attr_json.insert(
                                "ObjectIdentifier".to_string(),
                                serde_json::Value::String(attr.object_identifier.clone()),
                            );
                            attr_json.insert(
                                "Value".to_string(),
                                serde_json::Value::String(attr.value.clone()),
                            );
                            serde_json::Value::Object(attr_json)
                        })
                        .collect();
                    subject_json.insert(
                        "CustomAttributes".to_string(),
                        serde_json::Value::Array(custom_attrs),
                    );
                }

                config_json.insert(
                    "Subject".to_string(),
                    serde_json::Value::Object(subject_json),
                );
            }

            if let Some(csr_extensions) = &certificate_authority_configuration.csr_extensions {
                let mut csr_ext_json = serde_json::Map::new();

                if let Some(key_usage) = &csr_extensions.key_usage {
                    let mut key_usage_json = serde_json::Map::new();
                    key_usage_json.insert(
                        "DigitalSignature".to_string(),
                        serde_json::Value::Bool(key_usage.digital_signature),
                    );
                    key_usage_json.insert(
                        "NonRepudiation".to_string(),
                        serde_json::Value::Bool(key_usage.non_repudiation),
                    );
                    key_usage_json.insert(
                        "KeyEncipherment".to_string(),
                        serde_json::Value::Bool(key_usage.key_encipherment),
                    );
                    key_usage_json.insert(
                        "DataEncipherment".to_string(),
                        serde_json::Value::Bool(key_usage.data_encipherment),
                    );
                    key_usage_json.insert(
                        "KeyAgreement".to_string(),
                        serde_json::Value::Bool(key_usage.key_agreement),
                    );
                    key_usage_json.insert(
                        "KeyCertSign".to_string(),
                        serde_json::Value::Bool(key_usage.key_cert_sign),
                    );
                    key_usage_json.insert(
                        "CrlSign".to_string(),
                        serde_json::Value::Bool(key_usage.crl_sign),
                    );
                    key_usage_json.insert(
                        "EncipherOnly".to_string(),
                        serde_json::Value::Bool(key_usage.encipher_only),
                    );
                    key_usage_json.insert(
                        "DecipherOnly".to_string(),
                        serde_json::Value::Bool(key_usage.decipher_only),
                    );
                    csr_ext_json.insert(
                        "KeyUsage".to_string(),
                        serde_json::Value::Object(key_usage_json),
                    );
                }

                if let Some(subject_information_access) = &csr_extensions.subject_information_access
                {
                    let sia_array: Vec<serde_json::Value> = subject_information_access
                        .iter()
                        .map(|access| {
                            let mut access_json = serde_json::Map::new();
                            if let Some(access_method) = &access.access_method {
                                if let Some(access_method_type) = &access_method.access_method_type
                                {
                                    access_json.insert(
                                        "AccessMethodType".to_string(),
                                        serde_json::Value::String(format!(
                                            "{:?}",
                                            access_method_type
                                        )),
                                    );
                                }
                                if let Some(custom_object_identifier) =
                                    &access_method.custom_object_identifier
                                {
                                    access_json.insert(
                                        "CustomObjectIdentifier".to_string(),
                                        serde_json::Value::String(custom_object_identifier.clone()),
                                    );
                                }
                            }
                            if let Some(access_location) = &access.access_location {
                                if let Some(uniform_resource_identifier) =
                                    &access_location.uniform_resource_identifier
                                {
                                    access_json.insert(
                                        "UniformResourceIdentifier".to_string(),
                                        serde_json::Value::String(
                                            uniform_resource_identifier.clone(),
                                        ),
                                    );
                                }
                            }
                            serde_json::Value::Object(access_json)
                        })
                        .collect();
                    csr_ext_json.insert(
                        "SubjectInformationAccess".to_string(),
                        serde_json::Value::Array(sia_array),
                    );
                }

                config_json.insert(
                    "CsrExtensions".to_string(),
                    serde_json::Value::Object(csr_ext_json),
                );
            }

            json.insert(
                "CertificateAuthorityConfiguration".to_string(),
                serde_json::Value::Object(config_json),
            );
        }

        if let Some(revocation_configuration) = &ca.revocation_configuration {
            let mut rev_config_json = serde_json::Map::new();

            if let Some(crl_configuration) = &revocation_configuration.crl_configuration {
                let mut crl_json = serde_json::Map::new();
                crl_json.insert(
                    "Enabled".to_string(),
                    serde_json::Value::Bool(crl_configuration.enabled),
                );
                if let Some(expiration_in_days) = crl_configuration.expiration_in_days {
                    crl_json.insert(
                        "ExpirationInDays".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(expiration_in_days)),
                    );
                }
                if let Some(custom_cname) = &crl_configuration.custom_cname {
                    crl_json.insert(
                        "CustomCname".to_string(),
                        serde_json::Value::String(custom_cname.clone()),
                    );
                }
                if let Some(s3_bucket_name) = &crl_configuration.s3_bucket_name {
                    crl_json.insert(
                        "S3BucketName".to_string(),
                        serde_json::Value::String(s3_bucket_name.clone()),
                    );
                }
                if let Some(s3_object_acl) = &crl_configuration.s3_object_acl {
                    crl_json.insert(
                        "S3ObjectAcl".to_string(),
                        serde_json::Value::String(format!("{:?}", s3_object_acl)),
                    );
                }
                rev_config_json.insert(
                    "CrlConfiguration".to_string(),
                    serde_json::Value::Object(crl_json),
                );
            }

            if let Some(ocsp_configuration) = &revocation_configuration.ocsp_configuration {
                let mut ocsp_json = serde_json::Map::new();
                ocsp_json.insert(
                    "Enabled".to_string(),
                    serde_json::Value::Bool(ocsp_configuration.enabled),
                );
                if let Some(ocsp_custom_cname) = &ocsp_configuration.ocsp_custom_cname {
                    ocsp_json.insert(
                        "OcspCustomCname".to_string(),
                        serde_json::Value::String(ocsp_custom_cname.clone()),
                    );
                }
                rev_config_json.insert(
                    "OcspConfiguration".to_string(),
                    serde_json::Value::Object(ocsp_json),
                );
            }

            json.insert(
                "RevocationConfiguration".to_string(),
                serde_json::Value::Object(rev_config_json),
            );
        }

        // Note: restore_date field doesn't exist in the current SDK version

        if let Some(key_storage_security_standard) = &ca.key_storage_security_standard {
            json.insert(
                "KeyStorageSecurityStandard".to_string(),
                serde_json::Value::String(format!("{:?}", key_storage_security_standard)),
            );
        }

        if let Some(usage_mode) = &ca.usage_mode {
            json.insert(
                "UsageMode".to_string(),
                serde_json::Value::String(format!("{:?}", usage_mode)),
            );
        }

        // Set default name if not available from common name
        if !json.contains_key("Name") {
            if let Some(arn) = &ca.arn {
                if let Some(ca_id) = arn.split('/').next_back() {
                    json.insert(
                        "Name".to_string(),
                        serde_json::Value::String(format!("CA-{}", ca_id)),
                    );
                }
            } else {
                json.insert(
                    "Name".to_string(),
                    serde_json::Value::String("unknown-ca".to_string()),
                );
            }
        }

        serde_json::Value::Object(json)
    }

    fn certificate_authority_detail_to_json(
        &self,
        ca: &acmpca::types::CertificateAuthority,
    ) -> serde_json::Value {
        // For detailed view, we use the same conversion as the summary since AWS PCA
        // DescribeCertificateAuthority returns the same CertificateAuthority structure
        self.certificate_authority_to_json(ca)
    }
}
