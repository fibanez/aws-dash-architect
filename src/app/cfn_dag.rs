//! CloudFormation resource dependency graph management and validation.
//!
//! This module provides a directed acyclic graph (DAG) system for managing CloudFormation
//! resource dependencies, enabling automated validation, topological sorting, and visualization
//! of infrastructure relationships. The DAG ensures deployment ordering correctness and
//! prevents circular dependencies that would cause CloudFormation stack failures.
//!
//! # Core Components
//!
//! - [`ResourceDag`] - Main graph structure managing resources and their dependencies
//! - Dependency validation with cycle detection using depth-first search
//! - Smart dependency resolution for out-of-order resource processing
//! - Topological sorting for optimal deployment ordering
//! - Emergency recovery methods for corrupted graph states
//!
//! # Architecture Benefits
//!
//! The graph-based approach provides several key advantages over linear resource processing:
//!
//! - **Dependency Validation**: Detect circular dependencies before deployment
//! - **Optimal Ordering**: Calculate correct deployment sequence automatically
//! - **Template Synchronization**: Keep graph state aligned with CloudFormation templates
//! - **Smart Import**: Handle resources regardless of declaration order in templates
//! - **Visual Layout**: Support node positioning for graphical dependency visualization
//!
//! # Integration with Template System
//!
//! The DAG integrates tightly with the CloudFormation template system to extract both
//! explicit dependencies (DependsOn) and implicit dependencies (Ref, GetAtt functions).
//! This comprehensive dependency analysis ensures all resource relationships are captured
//! and validated before deployment.
//!
//! # Algorithms and Reliability
//!
//! The module uses proven graph algorithms for correctness:
//!
//! - **Cycle Detection**: Depth-first search with recursion stack tracking
//! - **Topological Sort**: Kahn's algorithm for dependency-ordered processing
//! - **Smart Resolution**: Deferred queue processing for dependency-aware imports
//! - **Recovery Patterns**: Emergency methods for graph state reconstruction
//!
//! See [dependency graph system documentation](../../../docs/technical/dependency-graph-system.wiki)
//! for implementation details and usage patterns.

use crate::app::cfn_template::CloudFormationTemplate;
use crate::app::projects::{CloudFormationResource, ResourceNode};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use tracing::{debug, info, warn};

/// Directed Acyclic Graph for CloudFormation resource dependency management.
///
/// `ResourceDag` provides comprehensive dependency analysis and validation for CloudFormation
/// resources, ensuring correct deployment ordering and preventing circular dependencies.
/// The graph maintains both explicit dependencies (DependsOn) and implicit dependencies
/// (Ref, GetAtt) extracted from CloudFormation templates.
///
/// # Core Responsibilities
///
/// - **Dependency Validation**: Verify all resource dependencies exist and detect cycles
/// - **Smart Import Processing**: Handle resources in any order through deferred queue processing
/// - **Deployment Ordering**: Calculate optimal resource deployment sequence via topological sort
/// - **Template Synchronization**: Maintain consistency with CloudFormation template changes
/// - **Visualization Support**: Manage node positioning for graphical dependency displays
/// - **Error Recovery**: Provide emergency methods for corrupted graph state recovery
///
/// # Graph Algorithms
///
/// The DAG uses proven computer science algorithms for reliability:
///
/// - **Cycle Detection**: Depth-first search with recursion stack to detect circular dependencies
/// - **Topological Sorting**: Kahn's algorithm for dependency-ordered resource sequences
/// - **Smart Resolution**: Queue-based processing for handling missing dependencies during import
///
/// # Usage Examples
///
/// Create and populate a dependency graph:
/// ```rust
/// use crate::app::cfn_dag::ResourceDag;
/// use crate::app::cfn_template::CloudFormationTemplate;
///
/// let mut dag = ResourceDag::new();
/// let template = CloudFormationTemplate::load_from_file("template.json")?;
/// let resources = template.extract_resources()?;
///
/// // Smart import handles dependency ordering automatically
/// let (added_count, warnings) = dag.add_resources_from_template(&template, resources)?;
/// ```
///
/// Validate deployment ordering:
/// ```rust
/// // Get optimal deployment sequence
/// let deployment_order = dag.get_deployment_order()?;
///
/// // Validate against template
/// let validation_errors = dag.validate_against_template(&template);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDag {
    /// Map of resource IDs to their dependency graph nodes.
    /// Each node contains the resource ID and list of dependencies.
    nodes: HashMap<String, ResourceNode>,

    /// Map of resource IDs to their CloudFormation resource data.
    /// Contains the full resource definition including properties and metadata.
    resources: HashMap<String, CloudFormationResource>,

    /// Node positions for graphical visualization layout.
    /// Maps resource IDs to (x, y) coordinates for dependency graph display.
    node_positions: HashMap<String, (f32, f32)>,

    /// Queue for deferred resources awaiting dependency resolution.
    /// Resources with missing dependencies are queued here for retry processing.
    deferred_queue: VecDeque<(CloudFormationResource, Vec<String>)>,
}

impl Default for ResourceDag {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceDag {
    /// Create a new empty dependency graph ready for resource management.
    ///
    /// Initializes all internal collections for managing CloudFormation resources
    /// and their dependencies. The graph starts empty and can handle any number
    /// of resources through smart dependency resolution.
    ///
    /// # Returns
    ///
    /// A new `ResourceDag` instance with no resources or dependencies.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::app::cfn_dag::ResourceDag;
    ///
    /// let mut dag = ResourceDag::new();
    /// assert_eq!(dag.get_resources().len(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            resources: HashMap::new(),
            node_positions: HashMap::new(),
            deferred_queue: VecDeque::new(),
        }
    }

    /// Emergency method to add a resource bypassing all dependency validations.
    ///
    /// This method directly inserts a resource into the graph without checking dependencies,
    /// cycles, or any other constraints. It's designed for emergency recovery scenarios
    /// where the graph state needs reconstruction after corruption or import failures.
    ///
    /// # Safety and Usage
    ///
    /// This method should only be used when:
    /// - Normal `add_resource` methods have failed due to dependency issues
    /// - Recovering from a corrupted graph state during emergency operations
    /// - Implementing advanced graph reconstruction algorithms
    ///
    /// # Arguments
    ///
    /// * `resource` - The CloudFormation resource to add directly to the graph
    ///
    /// # Side Effects
    ///
    /// - Creates a graph node with no dependencies (empty depends_on list)
    /// - Assigns default positioning for visualization (150px grid spacing)
    /// - Logs the emergency addition for debugging and audit purposes
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::app::cfn_dag::ResourceDag;
    /// use crate::app::projects::CloudFormationResource;
    ///
    /// let mut dag = ResourceDag::new();
    /// let resource = CloudFormationResource {
    ///     resource_id: "MyS3Bucket".to_string(),
    ///     resource_type: "AWS::S3::Bucket".to_string(),
    ///     properties: HashMap::new(),
    /// };
    ///
    /// // Emergency addition without dependency validation
    /// dag.direct_add_resource(resource);
    /// ```
    ///
    /// # Warning
    ///
    /// Using this method may result in an invalid graph state with:
    /// - Missing dependency relationships
    /// - Potential circular dependencies
    /// - Incorrect deployment ordering
    ///
    /// Always validate the graph state after emergency operations.
    pub fn direct_add_resource(&mut self, resource: CloudFormationResource) {
        let resource_id_for_log = resource.resource_id.clone();

        // Add to resources map
        self.resources
            .insert(resource.resource_id.clone(), resource.clone());

        // Create a node for it
        let node = ResourceNode {
            resource_id: resource.resource_id.clone(),
            depends_on: Vec::new(),
        };

        // Add the node
        self.nodes.insert(resource.resource_id.clone(), node);

        // Assign a default position if not already set
        if !self.node_positions.contains_key(&resource.resource_id) {
            // Find a reasonable default position
            let x = (self.nodes.len() as f32 * 150.0) % 800.0;
            let y = (self.nodes.len() as f32 / 5.0).floor() * 150.0;
            self.node_positions
                .insert(resource.resource_id.clone(), (x, y));
        }

        tracing::debug!(
            "Direct-added resource {} to DAG (bypass validation)",
            resource_id_for_log
        );
    }

    /// Emergency method to replace entire graph state with new collections.
    ///
    /// This method completely replaces all internal collections with new data,
    /// bypassing all validation and dependency checks. It's designed for emergency
    /// recovery scenarios where the entire graph needs reconstruction from known-good
    /// collections or when implementing bulk import operations.
    ///
    /// # Safety and Usage
    ///
    /// Use this method when:
    /// - Recovering from complete graph corruption
    /// - Implementing bulk data import from external sources
    /// - Restoring graph state from serialized backup data
    /// - Performing advanced graph reconstruction operations
    ///
    /// # Arguments
    ///
    /// * `resources` - New resource collection to replace current resources
    /// * `node_positions` - New positioning data for visualization layout
    ///
    /// # Side Effects
    ///
    /// - Completely replaces all existing graph data
    /// - Creates minimal dependency nodes (empty depends_on lists)
    /// - Clears the deferred queue to start fresh processing
    /// - Logs the replacement operation for audit tracking
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::collections::HashMap;
    /// use crate::app::cfn_dag::ResourceDag;
    /// use crate::app::projects::CloudFormationResource;
    ///
    /// let mut dag = ResourceDag::new();
    /// let resources = HashMap::new(); // Load from backup
    /// let positions = HashMap::new(); // Load layout data
    ///
    /// // Emergency replacement of entire graph state
    /// dag.replace_collections(resources, positions);
    /// ```
    ///
    /// # Warning
    ///
    /// This method may result in:
    /// - Loss of all existing dependency relationships
    /// - Inconsistent graph state requiring validation
    /// - Missing dependencies that need reconstruction
    ///
    /// Always validate and rebuild dependencies after using this method.
    pub fn replace_collections(
        &mut self,
        resources: HashMap<String, CloudFormationResource>,
        node_positions: HashMap<String, (f32, f32)>,
    ) {
        // Create nodes from resources
        let mut nodes = HashMap::new();
        for resource_id in resources.keys() {
            nodes.insert(
                resource_id.clone(),
                ResourceNode {
                    resource_id: resource_id.clone(),
                    depends_on: Vec::new(),
                },
            );
        }

        // Replace collections
        self.resources = resources;
        self.nodes = nodes;
        self.node_positions = node_positions;
        self.deferred_queue.clear(); // Clear deferred queue on full replacement

        tracing::warn!(
            "All DAG collections replaced with {} resources",
            self.resources.len()
        );
    }

    /// Add a resource to the DAG
    pub fn add_resource(
        &mut self,
        resource: CloudFormationResource,
        depends_on: Vec<String>,
    ) -> Result<()> {
        let resource_id = resource.resource_id.clone();

        // TODO: CRITICAL DEPENDENCY RESOLUTION BUG
        //
        // PROBLEM: Strict dependency validation causes resource import failures
        // When importing CloudFormation templates, resources with dependencies
        // fail to import if their dependencies haven't been processed yet.
        //
        // CURRENT FAILURES (from logs):
        // - ConfigRuleForVolumeAutoEnableIO fails: Dependency ConfigPermissionToCallLambda not found
        // - ConfigRuleForVolumeTags fails: Dependency ConfigRecorder not found
        //
        // ROOT CAUSE: Resources are processed in template iteration order, not dependency order
        // If Resource A depends on Resource B, but B appears later in the template, A fails
        //
        // ALGORITHM NEEDED: Implement smart dependency resolution
        // 1. Queue resources that fail dependency validation
        // 2. Retry queued resources after each successful addition
        // 3. Use topological sorting for optimal import order
        // 4. Handle circular dependencies gracefully
        // 5. Import all resources regardless of dependency order

        // Validate dependencies exist
        for dep_id in &depends_on {
            if !self.nodes.contains_key(dep_id) && !dep_id.is_empty() {
                return Err(anyhow!("Dependency {} not found", dep_id)); // <- FAILS HERE: Resource gets skipped
            }
        }

        // Check for cycles if adding this dependency
        let mut temp_nodes = self.nodes.clone();
        temp_nodes.insert(
            resource_id.clone(),
            ResourceNode {
                resource_id: resource_id.clone(),
                depends_on: depends_on.clone(),
            },
        );

        if Self::has_cycle(&temp_nodes) {
            return Err(anyhow!("Adding this dependency would create a cycle"));
        }

        // Add the node and resource
        self.nodes.insert(
            resource_id.clone(),
            ResourceNode {
                resource_id: resource_id.clone(),
                depends_on: depends_on.clone(), // Clone here to avoid move
            },
        );

        self.resources.insert(resource_id.clone(), resource);

        // Assign a default position if not already set
        if !self.node_positions.contains_key(&resource_id) {
            // Find a reasonable default position
            let x = (self.nodes.len() as f32 * 150.0) % 800.0;
            let y = (self.nodes.len() as f32 / 5.0).floor() * 150.0;
            let resource_id_for_position = resource_id.clone(); // Clone for use in insert
            tracing::info!("🎯 DEFAULT_POSITION: Assigning default position to {}: ({:.1}, {:.1}) based on node count {}",
                resource_id, x, y, self.nodes.len());
            self.node_positions.insert(resource_id_for_position, (x, y));
        }

        debug!("Added resource {} to DAG", &resource_id);
        Ok(())
    }

    /// Add a resource with intelligent dependency resolution and deferred processing.
    ///
    /// This method provides smart handling for resources with dependencies that may not
    /// exist yet in the graph. Instead of failing immediately, resources with missing
    /// dependencies are queued for later processing, enabling successful import of
    /// CloudFormation templates regardless of resource declaration order.
    ///
    /// # Dependency Resolution Algorithm
    ///
    /// 1. **Direct Addition**: Attempt immediate resource addition if all dependencies exist
    /// 2. **Deferred Queuing**: Queue resources with missing dependencies for retry processing
    /// 3. **Cascade Processing**: Process deferred queue after each successful addition
    /// 4. **Progress Tracking**: Return count of resources successfully added
    ///
    /// # Arguments
    ///
    /// * `resource` - The CloudFormation resource to add to the graph
    /// * `depends_on` - List of dependency resource IDs this resource depends on
    ///
    /// # Returns
    ///
    /// The number of resources successfully added (including cascaded deferred resources).
    /// Returns 0 if the resource was queued for later processing.
    ///
    /// # Errors
    ///
    /// Returns error for non-recoverable issues:
    /// - Circular dependency detection (would create invalid graph)
    /// - Graph corruption or internal consistency errors
    ///
    /// Does NOT error for missing dependencies - these trigger deferred processing.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::app::cfn_dag::ResourceDag;
    /// use crate::app::projects::CloudFormationResource;
    ///
    /// let mut dag = ResourceDag::new();
    /// let resource = CloudFormationResource {
    ///     resource_id: "WebServer".to_string(),
    ///     resource_type: "AWS::EC2::Instance".to_string(),
    ///     properties: HashMap::new(),
    /// };
    ///
    /// // Add resource that depends on security group (may not exist yet)
    /// let added_count = dag.add_resource_smart(
    ///     resource,
    ///     vec!["WebServerSecurityGroup".to_string()]
    /// )?;
    ///
    /// if added_count == 0 {
    ///     println!("Resource queued for deferred processing");
    /// }
    /// ```
    ///
    /// # Benefits
    ///
    /// - **Order Independence**: Handle resources in any declaration order
    /// - **Batch Processing**: Efficiently process related resources together
    /// - **Automatic Retry**: Intelligently retry failed resources as dependencies become available
    /// - **Progress Reporting**: Track how many resources are successfully processed
    pub fn add_resource_smart(
        &mut self,
        resource: CloudFormationResource,
        depends_on: Vec<String>,
    ) -> Result<usize> {
        let resource_id = resource.resource_id.clone();

        // Try to add the resource directly first
        match self.add_resource(resource.clone(), depends_on.clone()) {
            Ok(()) => {
                info!("Successfully added resource {} directly", resource_id);
                // Try to process any deferred resources that might now be ready
                let processed = self.process_deferred_queue()?;
                Ok(1 + processed) // Return count of resources added
            }
            Err(e) => {
                // If it failed due to missing dependencies, queue it for later
                if e.to_string().contains("Dependency") && e.to_string().contains("not found") {
                    warn!(
                        "Deferring resource {} due to missing dependencies: {}",
                        resource_id, e
                    );
                    self.deferred_queue.push_back((resource, depends_on));
                    Ok(0) // No resources added yet
                } else {
                    // Other errors (like cycles) should still fail
                    Err(e)
                }
            }
        }
    }

    /// Process deferred resources whose dependencies may now be available.
    ///
    /// This method implements the core algorithm for smart dependency resolution by
    /// repeatedly attempting to process resources in the deferred queue. Resources
    /// are retried as dependencies become available, enabling efficient batch processing
    /// of interdependent CloudFormation resources.
    ///
    /// # Processing Algorithm
    ///
    /// 1. **Iterative Processing**: Continue processing until no progress is made
    /// 2. **Queue Cycling**: Attempt each deferred resource once per iteration
    /// 3. **Dependency Checking**: Retry resources when their dependencies are now available
    /// 4. **Progress Tracking**: Stop when no resources can be processed in a full cycle
    /// 5. **Error Handling**: Fail fast on non-recoverable errors (cycles, corruption)
    ///
    /// # Returns
    ///
    /// The number of deferred resources successfully processed and added to the graph.
    /// Returns 0 if no deferred resources were ready for processing.
    ///
    /// # Errors
    ///
    /// Returns error for:
    /// - Circular dependency detection during resource addition
    /// - Graph corruption or internal consistency errors
    /// - Resource validation failures not related to missing dependencies
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::app::cfn_dag::ResourceDag;
    ///
    /// let mut dag = ResourceDag::new();
    /// // ... add several resources with dependencies ...
    ///
    /// // Process any resources waiting for dependency resolution
    /// let processed = dag.process_deferred_queue()?;
    /// println!("Processed {} deferred resources", processed);
    ///
    /// // Check remaining unprocessed resources
    /// let remaining = dag.get_deferred_resource_ids();
    /// if !remaining.is_empty() {
    ///     println!("Resources still waiting: {:?}", remaining);
    /// }
    /// ```
    ///
    /// # Performance Characteristics
    ///
    /// - **Time Complexity**: O(n²) worst case where n is deferred queue size
    /// - **Convergence**: Terminates when no progress can be made
    /// - **Memory Usage**: Processes queue in-place without additional allocations
    /// - **Retry Logic**: Efficient retry without exponential backoff delays
    ///
    /// # Integration
    ///
    /// This method is automatically called by:
    /// - `add_resource_smart` after successful resource addition
    /// - `add_resources_from_template` for bulk processing operations
    /// - Manual calls when dependencies are added external to the DAG
    pub fn process_deferred_queue(&mut self) -> Result<usize> {
        let mut processed_count = 0;
        let mut made_progress = true;

        // Keep trying until we can't make any more progress
        while made_progress && !self.deferred_queue.is_empty() {
            made_progress = false;
            let queue_len = self.deferred_queue.len();

            // Try to process each item in the queue
            for _ in 0..queue_len {
                if let Some((resource, depends_on)) = self.deferred_queue.pop_front() {
                    let resource_id = resource.resource_id.clone();

                    match self.add_resource(resource.clone(), depends_on.clone()) {
                        Ok(()) => {
                            info!("Successfully processed deferred resource {}", resource_id);
                            processed_count += 1;
                            made_progress = true;
                        }
                        Err(e) => {
                            if e.to_string().contains("Dependency")
                                && e.to_string().contains("not found")
                            {
                                // Still missing dependencies, put it back in the queue
                                self.deferred_queue.push_back((resource, depends_on));
                            } else {
                                // Other errors (like cycles) should fail
                                warn!("Failed to process deferred resource {}: {}", resource_id, e);
                                return Err(e);
                            }
                        }
                    }
                } else {
                    break; // Queue is empty
                }
            }
        }

        Ok(processed_count)
    }

    /// Bulk import resources from CloudFormation template with comprehensive dependency resolution.
    ///
    /// This method provides the primary interface for importing entire CloudFormation templates
    /// into the dependency graph. It handles complex dependency relationships automatically,
    /// extracting both explicit (DependsOn) and implicit (Ref, GetAtt) dependencies to build
    /// a complete and accurate dependency graph.
    ///
    /// # Smart Import Algorithm
    ///
    /// 1. **Dependency Extraction**: Analyze each resource for all dependency types
    /// 2. **Smart Addition**: Use deferred processing for missing dependencies
    /// 3. **Cascade Processing**: Automatically retry deferred resources as dependencies become available
    /// 4. **Comprehensive Reporting**: Track success count and collect warnings for troubleshooting
    /// 5. **Final Resolution**: Attempt final processing of any remaining deferred resources
    ///
    /// # Arguments
    ///
    /// * `template` - CloudFormation template containing dependency information
    /// * `resources` - Vector of resources to add to the graph
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// - `usize`: Total number of resources successfully added to the graph
    /// - `Vec<String>`: Warning messages for any resources that couldn't be processed
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::app::cfn_dag::ResourceDag;
    /// use crate::app::cfn_template::CloudFormationTemplate;
    ///
    /// let mut dag = ResourceDag::new();
    /// let template = CloudFormationTemplate::load_from_file("complex-template.json")?;
    /// let resources = template.extract_resources()?;
    ///
    /// let (added_count, warnings) = dag.add_resources_from_template(&template, resources)?;
    ///
    /// println!("Successfully imported {} resources", added_count);
    /// for warning in warnings {
    ///     eprintln!("Warning: {}", warning);
    /// }
    ///
    /// // Verify deployment readiness
    /// let deployment_order = dag.get_deployment_order()?;
    /// println!("Deployment sequence: {:?}", deployment_order);
    /// ```
    ///
    /// # Benefits
    ///
    /// - **Order Independence**: Resources can appear in any order in the template
    /// - **Complete Dependency Analysis**: Captures all CloudFormation dependency types
    /// - **Robust Error Handling**: Provides detailed warnings for troubleshooting
    /// - **Progress Reporting**: Shows exactly how many resources were successfully processed
    /// - **Validation Ready**: Creates graph suitable for deployment ordering and cycle detection
    ///
    /// # Error Handling
    ///
    /// The method uses graceful error handling:
    /// - Individual resource failures are captured as warnings, not fatal errors
    /// - Dependency resolution continues even if some resources cannot be processed
    /// - Circular dependency detection prevents invalid graph states
    /// - Deferred resources are reported for manual intervention if needed
    ///
    /// # Performance
    ///
    /// - **Efficient Processing**: Single-pass template analysis with smart retry logic
    /// - **Memory Conscious**: Deferred queue prevents memory buildup for large templates
    /// - **Scalable**: Handles CloudFormation templates with hundreds of resources
    pub fn add_resources_from_template(
        &mut self,
        template: &CloudFormationTemplate,
        resources: Vec<CloudFormationResource>,
    ) -> Result<(usize, Vec<String>)> {
        let mut total_added = 0;
        let mut warnings = Vec::new();

        info!(
            "Adding {} resources with smart dependency resolution",
            resources.len()
        );

        // Add all resources, allowing for deferred processing
        for resource in resources {
            let resource_id = resource.resource_id.clone();

            // Extract dependencies for this resource
            let mut all_dependencies = Vec::new();
            if let Some(cfn_resource) = template.resources.get(&resource_id) {
                // Add explicit DependsOn dependencies
                if let Some(depends_on) = &cfn_resource.depends_on {
                    all_dependencies.extend(depends_on.to_vec());
                }

                // Extract implicit dependencies from Ref and GetAtt
                let implicit_deps = template.extract_implicit_dependencies(cfn_resource);
                all_dependencies.extend(implicit_deps);
            }

            // Remove duplicates
            all_dependencies.sort();
            all_dependencies.dedup();

            match self.add_resource_smart(resource, all_dependencies) {
                Ok(added_count) => {
                    total_added += added_count;
                }
                Err(e) => {
                    warnings.push(format!("Failed to add resource {}: {}", resource_id, e));
                }
            }
        }

        // Final attempt to process any remaining deferred resources
        match self.process_deferred_queue() {
            Ok(final_count) => {
                total_added += final_count;
            }
            Err(e) => {
                warnings.push(format!(
                    "Failed to process remaining deferred resources: {}",
                    e
                ));
            }
        }

        // Report any resources still in the deferred queue
        if !self.deferred_queue.is_empty() {
            let remaining_ids: Vec<String> = self
                .deferred_queue
                .iter()
                .map(|(resource, _)| resource.resource_id.clone())
                .collect();
            warnings.push(format!(
                "Warning: {} resources remain unprocessed due to unresolvable dependencies: {}",
                remaining_ids.len(),
                remaining_ids.join(", ")
            ));
        }

        info!(
            "Smart dependency resolution completed: {} resources added",
            total_added
        );
        Ok((total_added, warnings))
    }

    /// Get count of deferred resources
    pub fn get_deferred_count(&self) -> usize {
        self.deferred_queue.len()
    }

    /// Get IDs of deferred resources for debugging
    pub fn get_deferred_resource_ids(&self) -> Vec<String> {
        self.deferred_queue
            .iter()
            .map(|(resource, _)| resource.resource_id.clone())
            .collect()
    }

    /// Update the visual position of a resource node for dependency graph display.
    ///
    /// This method manages the positioning data used by the graphical dependency
    /// visualization system. Node positions are persisted with the graph and can
    /// be saved/loaded as part of project data.
    ///
    /// # Arguments
    ///
    /// * `resource_id` - The resource whose position should be updated
    /// * `x` - Horizontal coordinate in the visualization coordinate system
    /// * `y` - Vertical coordinate in the visualization coordinate system
    ///
    /// # Returns
    ///
    /// `Ok(())` if position was updated successfully.
    ///
    /// # Errors
    ///
    /// Returns error if the specified resource does not exist in the graph.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::app::cfn_dag::ResourceDag;
    ///
    /// let mut dag = ResourceDag::new();
    /// // ... add resources to dag ...
    ///
    /// // Position resource for clean visualization layout
    /// dag.update_node_position("WebServer", 100.0, 50.0)?;
    /// dag.update_node_position("Database", 200.0, 150.0)?;
    ///
    /// // Verify positions are stored
    /// let positions = dag.get_node_positions();
    /// assert_eq!(positions.get("WebServer"), Some(&(100.0, 50.0)));
    /// ```
    ///
    /// # Integration
    ///
    /// Position data is used by:
    /// - Dependency graph visualization window for node rendering
    /// - Project save/load operations for layout persistence
    /// - Automatic layout algorithms for initial positioning
    /// - User drag-and-drop operations in the graph editor
    pub fn update_node_position(&mut self, resource_id: &str, x: f32, y: f32) -> Result<()> {
        if !self.nodes.contains_key(resource_id) {
            tracing::warn!(
                "❌ DAG_POSITION: Resource {} not found in nodes, cannot update position",
                resource_id
            );
            return Err(anyhow!("Resource {} not found", resource_id));
        }

        let old_position = self.node_positions.get(resource_id).cloned();
        self.node_positions.insert(resource_id.to_string(), (x, y));

        if let Some((old_x, old_y)) = old_position {
            tracing::info!(
                "🔄 DAG_POSITION: Updated {} in DAG: ({:.1}, {:.1}) → ({:.1}, {:.1})",
                resource_id,
                old_x,
                old_y,
                x,
                y
            );
        } else {
            tracing::info!(
                "🆕 DAG_POSITION: Added new {} to DAG: ({:.1}, {:.1})",
                resource_id,
                x,
                y
            );
        }

        Ok(())
    }

    /// Get all resources
    pub fn get_resources(&self) -> &HashMap<String, CloudFormationResource> {
        &self.resources
    }

    /// Get a specific resource
    pub fn get_resource(&self, resource_id: &str) -> Option<&CloudFormationResource> {
        self.resources.get(resource_id)
    }

    /// Get all nodes
    pub fn get_nodes(&self) -> &HashMap<String, ResourceNode> {
        &self.nodes
    }

    /// Get node positions for visualization
    pub fn get_node_positions(&self) -> &HashMap<String, (f32, f32)> {
        &self.node_positions
    }

    /// Get dependencies for a resource
    pub fn get_dependencies(&self, resource_id: &str) -> Vec<String> {
        if let Some(node) = self.nodes.get(resource_id) {
            node.depends_on.clone()
        } else {
            Vec::new()
        }
    }

    /// Get resources that depend on the given resource
    pub fn get_dependents(&self, resource_id: &str) -> Vec<String> {
        let mut dependents = Vec::new();

        for (id, node) in &self.nodes {
            if node.depends_on.contains(&resource_id.to_string()) {
                dependents.push(id.clone());
            }
        }

        dependents
    }

    /// Remove a resource from the DAG
    pub fn remove_resource(&mut self, resource_id: &str) -> Result<()> {
        // Check if the resource exists
        if !self.nodes.contains_key(resource_id) {
            return Err(anyhow!("Resource {} not found", resource_id));
        }

        // Check if any resources depend on this one
        let dependents = self.get_dependents(resource_id);
        if !dependents.is_empty() {
            return Err(anyhow!(
                "Cannot remove resource {} because it is depended on by: {}",
                resource_id,
                dependents.join(", ")
            ));
        }

        // Remove the resource
        self.nodes.remove(resource_id);
        self.resources.remove(resource_id);
        self.node_positions.remove(resource_id);

        debug!("Removed resource {} from DAG", resource_id);
        Ok(())
    }

    /// Update a resource's dependencies
    pub fn update_dependencies(
        &mut self,
        resource_id: &str,
        depends_on: Vec<String>,
    ) -> Result<()> {
        // Check if the resource exists
        if !self.nodes.contains_key(resource_id) {
            return Err(anyhow!("Resource {} not found", resource_id));
        }

        // Validate dependencies exist
        for dep_id in &depends_on {
            if !self.nodes.contains_key(dep_id) && !dep_id.is_empty() {
                return Err(anyhow!("Dependency {} not found", dep_id));
            }
        }

        // Check for cycles if updating this dependency
        let mut temp_nodes = self.nodes.clone();
        if let Some(node) = temp_nodes.get_mut(resource_id) {
            node.depends_on = depends_on.clone();
        }

        if Self::has_cycle(&temp_nodes) {
            return Err(anyhow!("Updating this dependency would create a cycle"));
        }

        // Update the dependencies
        if let Some(node) = self.nodes.get_mut(resource_id) {
            node.depends_on = depends_on;
        }

        debug!("Updated dependencies for resource {}", resource_id);
        Ok(())
    }

    /// Update a resource's properties
    pub fn update_resource_properties(
        &mut self,
        resource_id: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        // Check if the resource exists
        if !self.resources.contains_key(resource_id) {
            return Err(anyhow!("Resource {} not found", resource_id));
        }

        // Update the properties
        if let Some(resource) = self.resources.get_mut(resource_id) {
            resource.properties = properties;
        }

        debug!("Updated properties for resource {}", resource_id);
        Ok(())
    }

    /// Detect circular dependencies using depth-first search algorithm.
    ///
    /// This method implements a depth-first search with recursion stack tracking
    /// to detect cycles in the dependency graph. Circular dependencies would prevent
    /// successful CloudFormation deployment and must be identified before attempting
    /// resource creation.
    ///
    /// # Algorithm: DFS Cycle Detection
    ///
    /// Uses the "white-gray-black" node coloring approach:
    /// - **White**: Unvisited nodes
    /// - **Gray**: Nodes currently in recursion stack (being processed)
    /// - **Black**: Completely processed nodes
    ///
    /// A cycle exists if we encounter a gray node during traversal.
    ///
    /// # Arguments
    ///
    /// * `nodes` - The graph nodes to check for cycles
    ///
    /// # Returns
    ///
    /// `true` if any circular dependency is detected, `false` if graph is acyclic.
    ///
    /// # Performance
    ///
    /// - **Time Complexity**: O(V + E) where V is nodes and E is dependency edges
    /// - **Space Complexity**: O(V) for visited and recursion stack tracking
    /// - **Early Termination**: Returns immediately upon detecting first cycle
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::collections::HashMap;
    /// use crate::app::cfn_dag::ResourceDag;
    /// use crate::app::projects::ResourceNode;
    ///
    /// let mut nodes = HashMap::new();
    /// nodes.insert("A".to_string(), ResourceNode {
    ///     resource_id: "A".to_string(),
    ///     depends_on: vec!["B".to_string()],
    /// });
    /// nodes.insert("B".to_string(), ResourceNode {
    ///     resource_id: "B".to_string(),
    ///     depends_on: vec!["A".to_string()], // Creates cycle: A -> B -> A
    /// });
    ///
    /// assert!(ResourceDag::has_cycle(&nodes)); // Detects A -> B -> A cycle
    /// ```
    fn has_cycle(nodes: &HashMap<String, ResourceNode>) -> bool {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for node_id in nodes.keys() {
            if Self::is_cyclic_util(nodes, node_id, &mut visited, &mut rec_stack) {
                return true;
            }
        }

        false
    }

    /// Recursive utility function implementing depth-first search cycle detection.
    ///
    /// This function performs the core recursion for cycle detection using the
    /// "white-gray-black" node coloring technique. It maintains both a visited set
    /// (for optimization) and a recursion stack (for cycle detection).
    ///
    /// # Algorithm Details
    ///
    /// 1. **Mark as Gray**: Add current node to recursion stack (currently processing)
    /// 2. **Visit Dependencies**: Recursively check all dependency nodes
    /// 3. **Cycle Check**: If dependency is in recursion stack, cycle detected
    /// 4. **Mark as Black**: Remove from recursion stack when done processing
    ///
    /// # Arguments
    ///
    /// * `nodes` - The complete graph node collection
    /// * `node_id` - Current node being processed
    /// * `visited` - Set of completely processed nodes (optimization)
    /// * `rec_stack` - Set of nodes currently in recursion stack (cycle detection)
    ///
    /// # Returns
    ///
    /// `true` if a cycle is detected from this node, `false` otherwise.
    ///
    /// # Implementation Notes
    ///
    /// - Skips empty dependency strings to handle self-references gracefully
    /// - Uses recursion stack to distinguish between visited and currently processing nodes
    /// - Optimizes by skipping already completely processed nodes
    /// - Properly cleans up recursion stack to prevent false positives
    fn is_cyclic_util(
        nodes: &HashMap<String, ResourceNode>,
        node_id: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> bool {
        // If node is not visited yet, mark it visited and add to recursion stack
        if !visited.contains(node_id) {
            visited.insert(node_id.to_string());
            rec_stack.insert(node_id.to_string());

            // Check all dependencies
            if let Some(node) = nodes.get(node_id) {
                for dep_id in &node.depends_on {
                    // Skip empty dependency strings (self-references)
                    if dep_id.is_empty() {
                        continue;
                    }

                    // If dependency is not visited, check it recursively
                    if !visited.contains(dep_id)
                        && Self::is_cyclic_util(nodes, dep_id, visited, rec_stack)
                    {
                        return true;
                    // If dependency is in recursion stack, there's a cycle
                    } else if rec_stack.contains(dep_id) {
                        return true;
                    }
                }
            }
        }

        // Remove from recursion stack and return false
        rec_stack.remove(node_id);
        false
    }

    /// Convert the DAG to a list of nodes for serialization
    pub fn to_node_list(&self) -> Vec<ResourceNode> {
        self.nodes.values().cloned().collect()
    }

    /// Convert the DAG to a list of resources for serialization
    pub fn to_resource_list(&self) -> Vec<CloudFormationResource> {
        self.resources.values().cloned().collect()
    }

    /// Add a resource with enhanced dependency analysis from CloudFormation template
    pub fn add_resource_with_template_validation(
        &mut self,
        resource: CloudFormationResource,
        template: &CloudFormationTemplate,
    ) -> Result<()> {
        let resource_id = resource.resource_id.clone();

        // Extract explicit dependencies from the resource
        let mut all_dependencies = Vec::new();

        // Add explicit DependsOn dependencies
        if let Some(cfn_resource) = template.resources.get(&resource_id) {
            if let Some(depends_on) = &cfn_resource.depends_on {
                all_dependencies.extend(depends_on.to_vec());
            }

            // Extract implicit dependencies from Ref and GetAtt
            let implicit_deps = template.extract_implicit_dependencies(cfn_resource);
            all_dependencies.extend(implicit_deps);
        }

        // Remove duplicates
        all_dependencies.sort();
        all_dependencies.dedup();

        // Validate all dependencies exist
        for dep_id in &all_dependencies {
            if !self.nodes.contains_key(dep_id) && !dep_id.is_empty() {
                return Err(anyhow!(
                    "Resource '{}' depends on non-existent resource '{}'",
                    resource_id,
                    dep_id
                ));
            }
        }

        // Add the resource with comprehensive dependencies
        self.add_resource(resource, all_dependencies)
    }

    /// Update dependencies based on CloudFormation template analysis
    pub fn update_dependencies_from_template(
        &mut self,
        resource_id: &str,
        template: &CloudFormationTemplate,
    ) -> Result<()> {
        // Check if the resource exists
        if !self.nodes.contains_key(resource_id) {
            return Err(anyhow!("Resource {} not found", resource_id));
        }

        let mut all_dependencies = Vec::new();

        if let Some(cfn_resource) = template.resources.get(resource_id) {
            // Add explicit DependsOn dependencies
            if let Some(depends_on) = &cfn_resource.depends_on {
                all_dependencies.extend(depends_on.to_vec());
            }

            // Extract implicit dependencies from Ref and GetAtt
            let implicit_deps = template.extract_implicit_dependencies(cfn_resource);
            all_dependencies.extend(implicit_deps);
        }

        // Remove duplicates
        all_dependencies.sort();
        all_dependencies.dedup();

        // Update the resource's dependencies
        self.update_dependencies(resource_id, all_dependencies)
    }

    /// Synchronize the entire DAG with a CloudFormation template
    pub fn synchronize_with_template(
        &mut self,
        template: &CloudFormationTemplate,
    ) -> Result<Vec<String>> {
        let mut warnings = Vec::new();

        // Validate all existing resources still exist in template
        let mut resources_to_remove = Vec::new();
        for resource_id in self.resources.keys() {
            if !template.resources.contains_key(resource_id) {
                resources_to_remove.push(resource_id.clone());
                warnings.push(format!(
                    "Resource '{}' exists in DAG but not in template - will be removed",
                    resource_id
                ));
            }
        }

        // Remove resources that no longer exist
        for resource_id in resources_to_remove {
            if let Err(e) = self.remove_resource(&resource_id) {
                warnings.push(format!(
                    "Failed to remove resource '{}' from DAG: {}",
                    resource_id, e
                ));
            }
        }

        // Update dependencies for all existing resources
        for resource_id in self.resources.keys().cloned().collect::<Vec<_>>() {
            if let Err(e) = self.update_dependencies_from_template(&resource_id, template) {
                warnings.push(format!(
                    "Failed to update dependencies for resource '{}': {}",
                    resource_id, e
                ));
            }
        }

        Ok(warnings)
    }

    /// Validate the DAG against a CloudFormation template
    pub fn validate_against_template(&self, template: &CloudFormationTemplate) -> Vec<String> {
        let mut errors = Vec::new();

        // Check template-level dependency validation
        errors.extend(template.validate_dependencies());
        errors.extend(template.detect_circular_dependencies());

        // Check DAG-specific validations
        for (resource_id, node) in &self.nodes {
            // Validate that all dependencies in the DAG exist in the template
            for dep_id in &node.depends_on {
                if !dep_id.is_empty() && !template.resources.contains_key(dep_id) {
                    errors.push(format!(
                        "DAG resource '{}' depends on '{}' which doesn't exist in template",
                        resource_id, dep_id
                    ));
                }
            }

            // Check for missing implicit dependencies
            if let Some(cfn_resource) = template.resources.get(resource_id) {
                let implicit_deps = template.extract_implicit_dependencies(cfn_resource);
                for implicit_dep in implicit_deps {
                    if !node.depends_on.contains(&implicit_dep) {
                        errors.push(format!(
                            "DAG resource '{}' is missing implicit dependency on '{}'",
                            resource_id, implicit_dep
                        ));
                    }
                }

                // Check for missing explicit dependencies
                if let Some(depends_on) = &cfn_resource.depends_on {
                    for explicit_dep in depends_on.to_vec() {
                        if !node.depends_on.contains(&explicit_dep) {
                            errors.push(format!(
                                "DAG resource '{}' is missing explicit dependency on '{}'",
                                resource_id, explicit_dep
                            ));
                        }
                    }
                }
            }
        }

        errors
    }

    /// Calculate optimal resource deployment ordering using topological sorting.
    ///
    /// This method implements Kahn's algorithm to determine the correct sequence for
    /// deploying CloudFormation resources based on their dependency relationships.
    /// The resulting order ensures that dependencies are always deployed before
    /// the resources that depend on them.
    ///
    /// # Algorithm: Kahn's Topological Sort
    ///
    /// 1. **In-Degree Calculation**: Count incoming dependencies for each resource
    /// 2. **Zero In-Degree Queue**: Start with resources having no dependencies
    /// 3. **Processing**: Remove resources from queue and decrease dependent in-degrees
    /// 4. **Cycle Detection**: Verify all resources are processed (no cycles remain)
    ///
    /// # Returns
    ///
    /// A vector of resource IDs in dependency-safe deployment order. Resources
    /// appearing earlier in the list can be deployed before those appearing later.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Circular dependencies exist (impossible to determine valid ordering)
    /// - Graph inconsistencies prevent proper sorting
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::app::cfn_dag::ResourceDag;
    ///
    /// let dag = ResourceDag::new();
    /// // ... populate with resources and dependencies ...
    ///
    /// match dag.get_deployment_order() {
    ///     Ok(order) => {
    ///         println!("Deploy resources in this order:");
    ///         for (i, resource_id) in order.iter().enumerate() {
    ///             println!("{}. {}", i + 1, resource_id);
    ///         }
    ///     }
    ///     Err(e) => {
    ///         eprintln!("Cannot determine deployment order: {}", e);
    ///         // Handle circular dependency
    ///     }
    /// }
    /// ```
    ///
    /// # Use Cases
    ///
    /// - **CloudFormation Deployment**: Ensure correct resource creation sequence
    /// - **Dependency Validation**: Verify graph has no circular dependencies
    /// - **Resource Planning**: Understand deployment complexity and parallelization opportunities
    /// - **Visualization**: Display resources in dependency-aware order
    ///
    /// # Performance
    ///
    /// - **Time Complexity**: O(V + E) where V is vertices (resources) and E is edges (dependencies)
    /// - **Space Complexity**: O(V) for in-degree tracking and processing queue
    /// - **Scalability**: Efficiently handles large CloudFormation templates with hundreds of resources
    pub fn get_deployment_order(&self) -> Result<Vec<String>> {
        let mut in_degree = HashMap::new();
        let mut adjacency_list: HashMap<String, Vec<String>> = HashMap::new();

        // Initialize in-degree count and adjacency list
        for resource_id in self.nodes.keys() {
            in_degree.insert(resource_id.clone(), 0);
            adjacency_list.insert(resource_id.clone(), Vec::new());
        }

        // Build adjacency list and calculate in-degrees
        for (resource_id, node) in &self.nodes {
            for dep_id in &node.depends_on {
                if !dep_id.is_empty() && self.nodes.contains_key(dep_id) {
                    // dep_id -> resource_id (dependency relationship)
                    adjacency_list
                        .get_mut(dep_id)
                        .unwrap()
                        .push(resource_id.clone());
                    *in_degree.get_mut(resource_id).unwrap() += 1;
                }
            }
        }

        let mut result = Vec::new();
        let mut queue = std::collections::VecDeque::new();

        // Add all nodes with in-degree 0 to queue
        for (resource_id, degree) in &in_degree {
            if *degree == 0 {
                queue.push_back(resource_id.clone());
            }
        }

        // Process the queue
        while let Some(resource_id) = queue.pop_front() {
            result.push(resource_id.clone());

            // For each dependent of current resource
            if let Some(dependents) = adjacency_list.get(&resource_id) {
                for dependent in dependents {
                    // Decrease in-degree
                    let degree = in_degree.get_mut(dependent).unwrap();
                    *degree -= 1;

                    // If in-degree becomes 0, add to queue
                    if *degree == 0 {
                        queue.push_back(dependent.clone());
                    }
                }
            }
        }

        // Check if all nodes are processed (no cycles)
        if result.len() != self.nodes.len() {
            return Err(anyhow!(
                "Circular dependency detected - cannot determine deployment order"
            ));
        }

        Ok(result)
    }

    /// Construct a dependency graph from separate node and resource collections.
    ///
    /// This factory method creates a `ResourceDag` from pre-existing collections,
    /// useful for deserializing saved project data or reconstructing graphs from
    /// external data sources. It automatically handles positioning assignment and
    /// collection mapping.
    ///
    /// # Arguments
    ///
    /// * `nodes` - Vector of dependency graph nodes containing resource relationships
    /// * `resources` - Vector of CloudFormation resources with their properties
    /// * `node_positions` - Optional positioning data for visualization layout
    ///
    /// # Returns
    ///
    /// A fully constructed `ResourceDag` with all collections populated and ready for use.
    ///
    /// # Automatic Layout
    ///
    /// If no position data is provided, the method automatically generates a grid layout:
    /// - 150 pixel spacing between nodes
    /// - 5 nodes per row maximum
    /// - Sequential positioning based on resource order
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::collections::HashMap;
    /// use crate::app::cfn_dag::ResourceDag;
    /// use crate::app::projects::{ResourceNode, CloudFormationResource};
    ///
    /// // Load from serialized project data
    /// let nodes = vec![
    ///     ResourceNode {
    ///         resource_id: "WebServer".to_string(),
    ///         depends_on: vec!["SecurityGroup".to_string()],
    ///     },
    ///     ResourceNode {
    ///         resource_id: "SecurityGroup".to_string(),
    ///         depends_on: vec![],
    ///     },
    /// ];
    ///
    /// let resources = vec![
    ///     CloudFormationResource {
    ///         resource_id: "WebServer".to_string(),
    ///         resource_type: "AWS::EC2::Instance".to_string(),
    ///         properties: HashMap::new(),
    ///     },
    ///     CloudFormationResource {
    ///         resource_id: "SecurityGroup".to_string(),
    ///         resource_type: "AWS::EC2::SecurityGroup".to_string(),
    ///         properties: HashMap::new(),
    ///     },
    /// ];
    ///
    /// let mut positions = HashMap::new();
    /// positions.insert("WebServer".to_string(), (200.0, 100.0));
    /// positions.insert("SecurityGroup".to_string(), (50.0, 100.0));
    ///
    /// let dag = ResourceDag::from_lists(nodes, resources, Some(positions));
    ///
    /// // Verify graph is ready for use
    /// assert_eq!(dag.get_resources().len(), 2);
    /// assert_eq!(dag.get_deployment_order().unwrap().len(), 2);
    /// ```
    ///
    /// # Use Cases
    ///
    /// - **Project Loading**: Reconstruct graphs from saved project files
    /// - **Data Import**: Convert external dependency data into graph format
    /// - **Testing**: Create known graph states for unit tests
    /// - **Backup Restoration**: Rebuild graphs from backup data
    pub fn from_lists(
        nodes: Vec<ResourceNode>,
        resources: Vec<CloudFormationResource>,
        node_positions: Option<HashMap<String, (f32, f32)>>,
    ) -> Self {
        let mut dag = Self::new();

        // Convert lists to maps
        for node in nodes {
            dag.nodes.insert(node.resource_id.clone(), node);
        }

        for resource in resources {
            dag.resources.insert(resource.resource_id.clone(), resource);
        }

        // Add positions if provided, otherwise create default positions
        if let Some(positions) = node_positions {
            dag.node_positions = positions;
        } else {
            // Create default positions based on node count
            for (i, resource_id) in dag.nodes.keys().enumerate() {
                let x = (i as f32 * 150.0) % 800.0;
                let y = (i as f32 / 5.0).floor() * 150.0;
                dag.node_positions.insert(resource_id.clone(), (x, y));
            }
        }

        dag
    }
}
