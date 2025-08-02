use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_cloudfront as cloudfront;
use std::sync::Arc;

pub struct CloudFrontService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl CloudFrontService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List CloudFront Distributions
    pub async fn list_distributions(
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

        let client = cloudfront::Client::new(&aws_config);

        let mut distributions = Vec::new();
        let mut marker = None;

        loop {
            let mut request = client.list_distributions();
            if let Some(ref marker_value) = marker {
                request = request.marker(marker_value);
            }

            let response = request.send().await?;

            if let Some(distribution_list) = response.distribution_list {
                if let Some(ref items) = distribution_list.items {
                    for dist_summary in items {
                        if let Ok(dist_details) = self
                            .get_distribution_internal(&client, &dist_summary.id)
                            .await
                        {
                            distributions.push(dist_details);
                        } else {
                            // Fallback to basic distribution info if get fails
                            let dist_json = self.distribution_summary_to_json(dist_summary);
                            distributions.push(dist_json);
                        }
                    }
                }

                if distribution_list.is_truncated {
                    marker = distribution_list.next_marker;
                    if marker.is_none() {
                        // If NextMarker is not provided but IsTruncated is true,
                        // use the last distribution ID as marker
                        if let Some(ref items) = distribution_list.items {
                            if let Some(last_item) = items.last() {
                                marker = Some(last_item.id.clone());
                            }
                        }
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        Ok(distributions)
    }

    /// Get detailed information for specific CloudFront Distribution
    pub async fn describe_distribution(
        &self,
        account_id: &str,
        region: &str,
        distribution_id: &str,
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

        let client = cloudfront::Client::new(&aws_config);
        self.get_distribution_internal(&client, distribution_id)
            .await
    }

    async fn get_distribution_internal(
        &self,
        client: &cloudfront::Client,
        distribution_id: &str,
    ) -> Result<serde_json::Value> {
        let response = client.get_distribution().id(distribution_id).send().await?;

        if let Some(distribution) = response.distribution {
            Ok(self.distribution_to_json(&distribution))
        } else {
            Err(anyhow::anyhow!(
                "Distribution {} not found",
                distribution_id
            ))
        }
    }

    fn distribution_summary_to_json(
        &self,
        dist_summary: &cloudfront::types::DistributionSummary,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "Id".to_string(),
            serde_json::Value::String(dist_summary.id.clone()),
        );
        json.insert(
            "ARN".to_string(),
            serde_json::Value::String(dist_summary.arn.clone()),
        );

        if let Some(aliases) = &dist_summary.aliases {
            if let Some(items) = &aliases.items {
                if !items.is_empty() {
                    json.insert(
                        "Name".to_string(),
                        serde_json::Value::String(items[0].clone()),
                    );
                    json.insert(
                        "DomainName".to_string(),
                        serde_json::Value::String(items.join(", ")),
                    );
                } else {
                    json.insert(
                        "Name".to_string(),
                        serde_json::Value::String(dist_summary.domain_name.clone()),
                    );
                    json.insert(
                        "DomainName".to_string(),
                        serde_json::Value::String(dist_summary.domain_name.clone()),
                    );
                }
            } else {
                json.insert(
                    "Name".to_string(),
                    serde_json::Value::String(dist_summary.domain_name.clone()),
                );
                json.insert(
                    "DomainName".to_string(),
                    serde_json::Value::String(dist_summary.domain_name.clone()),
                );
            }
        } else {
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(dist_summary.domain_name.clone()),
            );
            json.insert(
                "DomainName".to_string(),
                serde_json::Value::String(dist_summary.domain_name.clone()),
            );
        }

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(dist_summary.status.clone()),
        );
        json.insert(
            "LastModifiedTime".to_string(),
            serde_json::Value::String(dist_summary.last_modified_time.to_string()),
        );
        json.insert(
            "Comment".to_string(),
            serde_json::Value::String(dist_summary.comment.clone()),
        );
        json.insert(
            "Enabled".to_string(),
            serde_json::Value::Bool(dist_summary.enabled),
        );

        json.insert(
            "PriceClass".to_string(),
            serde_json::Value::String(format!("{:?}", dist_summary.price_class)),
        );

        json.insert(
            "HttpVersion".to_string(),
            serde_json::Value::String(format!("{:?}", dist_summary.http_version)),
        );

        json.insert(
            "IsIPV6Enabled".to_string(),
            serde_json::Value::Bool(dist_summary.is_ipv6_enabled),
        );

        json.insert(
            "WebACLId".to_string(),
            serde_json::Value::String(dist_summary.web_acl_id.clone()),
        );

        serde_json::Value::Object(json)
    }

    fn distribution_to_json(
        &self,
        distribution: &cloudfront::types::Distribution,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "Id".to_string(),
            serde_json::Value::String(distribution.id.clone()),
        );
        json.insert(
            "ARN".to_string(),
            serde_json::Value::String(distribution.arn.clone()),
        );
        json.insert(
            "Status".to_string(),
            serde_json::Value::String(distribution.status.clone()),
        );
        json.insert(
            "LastModifiedTime".to_string(),
            serde_json::Value::String(distribution.last_modified_time.to_string()),
        );
        json.insert(
            "InProgressInvalidationBatches".to_string(),
            serde_json::Value::Number(serde_json::Number::from(
                distribution.in_progress_invalidation_batches,
            )),
        );
        json.insert(
            "DomainName".to_string(),
            serde_json::Value::String(distribution.domain_name.clone()),
        );

        if let Some(active_trusted_signers) = &distribution.active_trusted_signers {
            json.insert(
                "ActiveTrustedSignersEnabled".to_string(),
                serde_json::Value::Bool(active_trusted_signers.enabled),
            );
            json.insert(
                "ActiveTrustedSignersQuantity".to_string(),
                serde_json::Value::Number(serde_json::Number::from(
                    active_trusted_signers.quantity,
                )),
            );
        }

        if let Some(active_trusted_key_groups) = &distribution.active_trusted_key_groups {
            json.insert(
                "ActiveTrustedKeyGroupsEnabled".to_string(),
                serde_json::Value::Bool(active_trusted_key_groups.enabled),
            );
            json.insert(
                "ActiveTrustedKeyGroupsQuantity".to_string(),
                serde_json::Value::Number(serde_json::Number::from(
                    active_trusted_key_groups.quantity,
                )),
            );
        }

        // Distribution Config
        if let Some(distribution_config) = &distribution.distribution_config {
            json.insert(
                "CallerReference".to_string(),
                serde_json::Value::String(distribution_config.caller_reference.clone()),
            );

            if let Some(aliases) = &distribution_config.aliases {
                if let Some(items) = &aliases.items {
                    if !items.is_empty() {
                        json.insert(
                            "Name".to_string(),
                            serde_json::Value::String(items[0].clone()),
                        );
                        json.insert(
                            "Aliases".to_string(),
                            serde_json::Value::Array(
                                items
                                    .iter()
                                    .map(|s| serde_json::Value::String(s.clone()))
                                    .collect(),
                            ),
                        );
                    } else {
                        json.insert(
                            "Name".to_string(),
                            serde_json::Value::String(distribution.domain_name.clone()),
                        );
                    }
                } else {
                    json.insert(
                        "Name".to_string(),
                        serde_json::Value::String(distribution.domain_name.clone()),
                    );
                }
            } else {
                json.insert(
                    "Name".to_string(),
                    serde_json::Value::String(distribution.domain_name.clone()),
                );
            }

            if let Some(default_root_object) = &distribution_config.default_root_object {
                json.insert(
                    "DefaultRootObject".to_string(),
                    serde_json::Value::String(default_root_object.clone()),
                );
            }

            if let Some(origins) = &distribution_config.origins {
                let origins_array: Vec<serde_json::Value> = origins
                    .items
                    .iter()
                    .map(|origin| {
                        let mut origin_json = serde_json::Map::new();
                        origin_json.insert(
                            "Id".to_string(),
                            serde_json::Value::String(origin.id.clone()),
                        );
                        origin_json.insert(
                            "DomainName".to_string(),
                            serde_json::Value::String(origin.domain_name.clone()),
                        );
                        if let Some(origin_path) = &origin.origin_path {
                            origin_json.insert(
                                "OriginPath".to_string(),
                                serde_json::Value::String(origin_path.clone()),
                            );
                        }
                        if let Some(s3_origin_config) = &origin.s3_origin_config {
                            origin_json.insert(
                                "OriginAccessIdentity".to_string(),
                                serde_json::Value::String(
                                    s3_origin_config.origin_access_identity.clone(),
                                ),
                            );
                        }
                        if let Some(custom_origin_config) = &origin.custom_origin_config {
                            origin_json.insert(
                                "HTTPPort".to_string(),
                                serde_json::Value::Number(serde_json::Number::from(
                                    custom_origin_config.http_port,
                                )),
                            );
                            origin_json.insert(
                                "HTTPSPort".to_string(),
                                serde_json::Value::Number(serde_json::Number::from(
                                    custom_origin_config.https_port,
                                )),
                            );
                            origin_json.insert(
                                "OriginProtocolPolicy".to_string(),
                                serde_json::Value::String(format!(
                                    "{:?}",
                                    custom_origin_config.origin_protocol_policy
                                )),
                            );
                            if let Some(ssl_protocols) = &custom_origin_config.origin_ssl_protocols
                            {
                                let ssl_protocols_array: Vec<serde_json::Value> = ssl_protocols
                                    .items
                                    .iter()
                                    .map(|protocol| {
                                        serde_json::Value::String(format!("{:?}", protocol))
                                    })
                                    .collect();
                                origin_json.insert(
                                    "OriginSslProtocols".to_string(),
                                    serde_json::Value::Array(ssl_protocols_array),
                                );
                            }
                        }
                        serde_json::Value::Object(origin_json)
                    })
                    .collect();
                json.insert(
                    "Origins".to_string(),
                    serde_json::Value::Array(origins_array),
                );
            }

            if let Some(default_cache_behavior) = &distribution_config.default_cache_behavior {
                let mut cache_behavior_json = serde_json::Map::new();
                cache_behavior_json.insert(
                    "TargetOriginId".to_string(),
                    serde_json::Value::String(default_cache_behavior.target_origin_id.clone()),
                );
                cache_behavior_json.insert(
                    "ViewerProtocolPolicy".to_string(),
                    serde_json::Value::String(format!(
                        "{:?}",
                        default_cache_behavior.viewer_protocol_policy
                    )),
                );
                if let Some(compress) = default_cache_behavior.compress {
                    cache_behavior_json
                        .insert("Compress".to_string(), serde_json::Value::Bool(compress));
                }

                if let Some(allowed_methods) = &default_cache_behavior.allowed_methods {
                    let methods_array: Vec<serde_json::Value> = allowed_methods
                        .items
                        .iter()
                        .map(|method| serde_json::Value::String(format!("{:?}", method)))
                        .collect();
                    cache_behavior_json.insert(
                        "AllowedMethods".to_string(),
                        serde_json::Value::Array(methods_array),
                    );
                }

                // Note: cached_methods might not be available in this context, skipping for now

                if let Some(cache_policy_id) = &default_cache_behavior.cache_policy_id {
                    cache_behavior_json.insert(
                        "CachePolicyId".to_string(),
                        serde_json::Value::String(cache_policy_id.clone()),
                    );
                }

                if let Some(origin_request_policy_id) =
                    &default_cache_behavior.origin_request_policy_id
                {
                    cache_behavior_json.insert(
                        "OriginRequestPolicyId".to_string(),
                        serde_json::Value::String(origin_request_policy_id.clone()),
                    );
                }

                json.insert(
                    "DefaultCacheBehavior".to_string(),
                    serde_json::Value::Object(cache_behavior_json),
                );
            }

            json.insert(
                "Comment".to_string(),
                serde_json::Value::String(distribution_config.comment.clone()),
            );
            json.insert(
                "Enabled".to_string(),
                serde_json::Value::Bool(distribution_config.enabled),
            );

            if let Some(price_class) = &distribution_config.price_class {
                json.insert(
                    "PriceClass".to_string(),
                    serde_json::Value::String(format!("{:?}", price_class)),
                );
            }

            if let Some(viewer_certificate) = &distribution_config.viewer_certificate {
                let mut cert_json = serde_json::Map::new();
                if let Some(cloudfront_default_certificate) =
                    viewer_certificate.cloud_front_default_certificate
                {
                    cert_json.insert(
                        "CloudFrontDefaultCertificate".to_string(),
                        serde_json::Value::Bool(cloudfront_default_certificate),
                    );
                }
                if let Some(iam_certificate_id) = &viewer_certificate.iam_certificate_id {
                    cert_json.insert(
                        "IAMCertificateId".to_string(),
                        serde_json::Value::String(iam_certificate_id.clone()),
                    );
                }
                if let Some(acm_certificate_arn) = &viewer_certificate.acm_certificate_arn {
                    cert_json.insert(
                        "ACMCertificateArn".to_string(),
                        serde_json::Value::String(acm_certificate_arn.clone()),
                    );
                }
                if let Some(ssl_support_method) = &viewer_certificate.ssl_support_method {
                    cert_json.insert(
                        "SSLSupportMethod".to_string(),
                        serde_json::Value::String(format!("{:?}", ssl_support_method)),
                    );
                }
                if let Some(minimum_protocol_version) = &viewer_certificate.minimum_protocol_version
                {
                    cert_json.insert(
                        "MinimumProtocolVersion".to_string(),
                        serde_json::Value::String(format!("{:?}", minimum_protocol_version)),
                    );
                }
                json.insert(
                    "ViewerCertificate".to_string(),
                    serde_json::Value::Object(cert_json),
                );
            }

            if let Some(web_acl_id) = &distribution_config.web_acl_id {
                json.insert(
                    "WebACLId".to_string(),
                    serde_json::Value::String(web_acl_id.clone()),
                );
            }

            if let Some(http_version) = &distribution_config.http_version {
                json.insert(
                    "HttpVersion".to_string(),
                    serde_json::Value::String(format!("{:?}", http_version)),
                );
            }

            if let Some(ipv6_enabled) = distribution_config.is_ipv6_enabled {
                json.insert(
                    "IsIPV6Enabled".to_string(),
                    serde_json::Value::Bool(ipv6_enabled),
                );
            }
        }

        serde_json::Value::Object(json)
    }
}
