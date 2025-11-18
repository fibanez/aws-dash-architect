use std::collections::HashMap;

/// Defines parent-child resource relationships for automatic recursive querying
pub struct ChildResourceConfig {
    /// Map of parent resource type -> list of child resource types
    parent_to_children: HashMap<String, Vec<ChildResourceDef>>,
}

pub struct ChildResourceDef {
    pub child_type: String,
    pub query_method: ChildQueryMethod,
}

pub enum ChildQueryMethod {
    /// Requires single parent ID parameter
    SingleParent { param_name: &'static str },
    /// Requires multiple parent parameters
    MultiParent { params: Vec<&'static str> },
}

impl Default for ChildResourceConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl ChildResourceConfig {
    pub fn new() -> Self {
        let mut parent_to_children = HashMap::new();

        // ============ Bedrock Knowledge Base Hierarchy ============

        // KnowledgeBase -> DataSource
        parent_to_children.insert(
            "AWS::Bedrock::KnowledgeBase".to_string(),
            vec![ChildResourceDef {
                child_type: "AWS::Bedrock::DataSource".to_string(),
                query_method: ChildQueryMethod::SingleParent {
                    param_name: "knowledge_base_id",
                },
            }],
        );

        // DataSource -> IngestionJob
        parent_to_children.insert(
            "AWS::Bedrock::DataSource".to_string(),
            vec![ChildResourceDef {
                child_type: "AWS::Bedrock::IngestionJob".to_string(),
                query_method: ChildQueryMethod::MultiParent {
                    params: vec!["knowledge_base_id", "data_source_id"],
                },
            }],
        );

        // ============ Bedrock Agent Hierarchy ============

        // Agent -> AgentAlias + AgentActionGroup
        parent_to_children.insert(
            "AWS::Bedrock::Agent".to_string(),
            vec![
                ChildResourceDef {
                    child_type: "AWS::Bedrock::AgentAlias".to_string(),
                    query_method: ChildQueryMethod::SingleParent {
                        param_name: "agent_id",
                    },
                },
                ChildResourceDef {
                    child_type: "AWS::Bedrock::AgentActionGroup".to_string(),
                    query_method: ChildQueryMethod::MultiParent {
                        params: vec!["agent_id", "agent_version"],
                    },
                },
            ],
        );

        // ============ Bedrock Flow Hierarchy ============

        // Flow -> FlowAlias
        parent_to_children.insert(
            "AWS::Bedrock::Flow".to_string(),
            vec![ChildResourceDef {
                child_type: "AWS::Bedrock::FlowAlias".to_string(),
                query_method: ChildQueryMethod::SingleParent {
                    param_name: "flow_id",
                },
            }],
        );

        Self {
            parent_to_children,
        }
    }

    /// Get child resource definitions for a parent resource type
    pub fn get_children(&self, parent_type: &str) -> Option<&[ChildResourceDef]> {
        self.parent_to_children
            .get(parent_type)
            .map(|v| v.as_slice())
    }

    /// Check if a resource type has children
    pub fn has_children(&self, parent_type: &str) -> bool {
        self.parent_to_children.contains_key(parent_type)
    }

    /// Get all parent resource types that have children
    pub fn get_all_parent_types(&self) -> Vec<&String> {
        self.parent_to_children.keys().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knowledge_base_has_children() {
        let config = ChildResourceConfig::new();
        assert!(config.has_children("AWS::Bedrock::KnowledgeBase"));

        let children = config.get_children("AWS::Bedrock::KnowledgeBase").unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].child_type, "AWS::Bedrock::DataSource");
    }

    #[test]
    fn test_data_source_has_children() {
        let config = ChildResourceConfig::new();
        assert!(config.has_children("AWS::Bedrock::DataSource"));

        let children = config.get_children("AWS::Bedrock::DataSource").unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].child_type, "AWS::Bedrock::IngestionJob");
    }

    #[test]
    fn test_agent_has_children() {
        let config = ChildResourceConfig::new();
        assert!(config.has_children("AWS::Bedrock::Agent"));

        let children = config.get_children("AWS::Bedrock::Agent").unwrap();
        assert_eq!(children.len(), 2);

        let child_types: Vec<&String> = children.iter().map(|c| &c.child_type).collect();
        assert!(child_types.contains(&&"AWS::Bedrock::AgentAlias".to_string()));
        assert!(child_types.contains(&&"AWS::Bedrock::AgentActionGroup".to_string()));
    }

    #[test]
    fn test_flow_has_children() {
        let config = ChildResourceConfig::new();
        assert!(config.has_children("AWS::Bedrock::Flow"));

        let children = config.get_children("AWS::Bedrock::Flow").unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].child_type, "AWS::Bedrock::FlowAlias");
    }

    #[test]
    fn test_model_has_no_children() {
        let config = ChildResourceConfig::new();
        assert!(!config.has_children("AWS::Bedrock::Model"));
        assert!(config.get_children("AWS::Bedrock::Model").is_none());
    }

    #[test]
    fn test_all_parent_types() {
        let config = ChildResourceConfig::new();
        let parents = config.get_all_parent_types();

        assert!(parents.contains(&&"AWS::Bedrock::KnowledgeBase".to_string()));
        assert!(parents.contains(&&"AWS::Bedrock::DataSource".to_string()));
        assert!(parents.contains(&&"AWS::Bedrock::Agent".to_string()));
        assert!(parents.contains(&&"AWS::Bedrock::Flow".to_string()));
    }
}
