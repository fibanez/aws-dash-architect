use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_iot as iot;
use std::sync::Arc;

pub struct IoTService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl IoTService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List IoT things
    pub async fn list_things(
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

        let client = iot::Client::new(&aws_config);
        let response = client.list_things().send().await?;

        let mut things = Vec::new();
        if let Some(thing_list) = response.things {
            for thing in thing_list {
                let thing_json = self.thing_to_json(&thing);
                things.push(thing_json);
            }
        }

        Ok(things)
    }

    /// Get detailed information for specific IoT thing
    pub async fn describe_thing(
        &self,
        account_id: &str,
        region: &str,
        thing_name: &str,
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

        let client = iot::Client::new(&aws_config);
        let response = client
            .describe_thing()
            .thing_name(thing_name)
            .send()
            .await?;

        let mut thing_details = serde_json::Map::new();

        if let Some(thing_name) = response.thing_name {
            thing_details.insert(
                "ThingName".to_string(),
                serde_json::Value::String(thing_name),
            );
        }

        if let Some(thing_id) = response.thing_id {
            thing_details.insert("ThingId".to_string(), serde_json::Value::String(thing_id));
        }

        if let Some(thing_arn) = response.thing_arn {
            thing_details.insert("ThingArn".to_string(), serde_json::Value::String(thing_arn));
        }

        if let Some(thing_type_name) = response.thing_type_name {
            thing_details.insert(
                "ThingTypeName".to_string(),
                serde_json::Value::String(thing_type_name),
            );
        }

        if let Some(attributes) = response.attributes {
            let attributes_map: serde_json::Map<String, serde_json::Value> = attributes
                .into_iter()
                .map(|(k, v)| (k, serde_json::Value::String(v)))
                .collect();
            thing_details.insert(
                "Attributes".to_string(),
                serde_json::Value::Object(attributes_map),
            );
        }

        thing_details.insert(
            "Version".to_string(),
            serde_json::Value::Number(response.version.into()),
        );

        if let Some(billing_group_name) = response.billing_group_name {
            thing_details.insert(
                "BillingGroupName".to_string(),
                serde_json::Value::String(billing_group_name),
            );
        }

        // Set status
        thing_details.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        Ok(serde_json::Value::Object(thing_details))
    }

    fn thing_to_json(&self, thing: &iot::types::ThingAttribute) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(thing_name) = &thing.thing_name {
            json.insert(
                "ThingName".to_string(),
                serde_json::Value::String(thing_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(thing_name.clone()),
            );
        }

        if let Some(thing_type_name) = &thing.thing_type_name {
            json.insert(
                "ThingTypeName".to_string(),
                serde_json::Value::String(thing_type_name.clone()),
            );
        }

        if let Some(thing_arn) = &thing.thing_arn {
            json.insert(
                "ThingArn".to_string(),
                serde_json::Value::String(thing_arn.clone()),
            );
            json.insert(
                "Arn".to_string(),
                serde_json::Value::String(thing_arn.clone()),
            );
        }

        if let Some(attributes) = &thing.attributes {
            let attributes_map: serde_json::Map<String, serde_json::Value> = attributes
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect();
            json.insert(
                "Attributes".to_string(),
                serde_json::Value::Object(attributes_map),
            );
        }

        json.insert(
            "Version".to_string(),
            serde_json::Value::Number(thing.version.into()),
        );

        // Set status
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        serde_json::Value::Object(json)
    }
}
