# AWS Cost Estimation & Scenario Planning Epic

## Epic Overview
Comprehensive cost estimation system that integrates with CloudFormation Manager to provide accurate AWS pricing analysis, scenario planning, and cost optimization recommendations.

## Research Summary & Key Findings

### AWS Pricing API Ecosystem (December 2024)
**Two Primary APIs:**
- **AWS Price List Bulk API (2015)**: Large structured JSON files, comprehensive but unwieldy
- **AWS Price List Query API (2017)**: RESTful granular queries, limited to 100 results per request

**Key Endpoints:**
- Bulk: `https://pricing.us-east-1.amazonaws.com`
- Query: `https://api.pricing.us-east-1.amazonaws.com`
- Service-specific: `https://pricing.us-east-1.amazonaws.com/offers/v1.0/aws/AmazonEC2/current/us-east-1/index.json`

**Critical API Limitations:**
- Only "AND" filter matching (no complex OR queries)
- Pagination tokens expire requiring restart
- Regional endpoint separation for pricing data
- Missing data: Free tier, spot pricing, limited offers
- Rate limiting and throttling restrictions
- IAM permissions required: `pricing:DescribeServices`, `pricing:GetAttributeValues`, `pricing:GetProducts`

### Service Pricing Complexity Examples

**EC2 Instance Pricing Dimensions:**
```json
{
  "attributes": {
    "clockSpeed": "2.3 GHz",
    "currentGeneration": "Yes", 
    "dedicatedEbsThroughput": "10000 Mbps",
    "ecu": "188",
    "enhancedNetworkingSupported": "Yes",
    "instanceFamily": "General purpose",
    "instanceType": "m4.16xlarge",
    "licenseModel": "No License required",
    "location": "Asia Pacific (Mumbai)",
    "locationType": "AWS Region",
    "memory": "256 GiB",
    "networkPerformance": "20 Gigabit",
    "operatingSystem": "Windows",
    "physicalProcessor": "Intel Xeon E5-2676 v3",
    "preInstalledSw": "SQL Ent",
    "processorArchitecture": "64-bit",
    "processorFeatures": "Intel AVX; Intel Turbo",
    "storage": "EBS only",
    "tenancy": "Shared",
    "usagetype": "APS1-BoxUsage:m4.16xlarge",
    "vcpu": "64"
  },
  "sku": "KPFD5NH6DG25VSCB",
  "pricing": {
    "pricePerUnit": {"USD": "30.8800000000"},
    "unit": "Hrs",
    "description": "$30.88 per On Demand Windows with SQL Server Enterprise m4.16xlarge Instance Hour"
  }
}
```

**RDS Pricing Dimensions:**
- Database engine, instance class, deployment option (Multi-AZ)
- Storage type (gp2, gp3, io1, io2), allocated storage, IOPS
- Backup storage, snapshot export, data transfer
- Reserved Instance terms, Savings Plans eligibility

**Lambda Pricing Dimensions:**
- Architecture (x86, ARM), memory allocation (128MB-10GB)
- Request count, duration, ephemeral storage (512MB-10GB)
- Provisioned concurrency, edge locations

**S3 Pricing Dimensions:**
- Storage class (Standard, IA, Glacier, Deep Archive)
- Request types (GET, PUT, DELETE, LIST), data retrieval
- Data transfer (in/out/between regions), transfer acceleration
- Management features (analytics, inventory, object tagging)

---

## Milestone 1: Core Pricing Infrastructure
**Goal**: Build robust AWS pricing data ingestion, caching, and query system

### Tasks

#### 1.1 AWS Pricing API Client
- [ ] Create `src/app/pricing/aws_pricing_client.rs`
- [ ] Implement Bulk API client with JSON file download/parsing
- [ ] Implement Query API client with pagination handling
- [ ] Add automatic retry logic with exponential backoff
- [ ] Handle rate limiting and token expiration gracefully
- [ ] Support regional endpoint switching

**Implementation Notes:**
```rust
pub struct AwsPricingClient {
    credential_coordinator: Arc<CredentialCoordinator>,
    bulk_cache: Arc<RwLock<HashMap<String, BulkPricingData>>>,
    query_cache: Arc<RwLock<HashMap<String, QueryResult>>>,
    cache_ttl: Duration, // 24 hours for bulk, 1 hour for query
}

pub struct BulkPricingData {
    service_code: String,
    last_updated: DateTime<Utc>,
    products: HashMap<String, Product>, // SKU -> Product
    terms: HashMap<String, PricingTerms>,
}
```

#### 1.2 Pricing Data Cache System
- [ ] Create `src/app/pricing/pricing_cache.rs`
- [ ] Implement multi-tier caching (memory + disk)
- [ ] Add cache invalidation based on AWS pricing update schedules
- [ ] Support partial cache updates for frequently changed services
- [ ] Add cache compression for bulk data storage

**Cache Strategy:**
- Memory: Hot pricing data (current region, common services)
- Disk: Full bulk pricing files (compressed JSON)
- TTL: 24h for bulk data, 1h for query results, 15min for spot prices

#### 1.3 Service-to-Pricing Mapping Engine
- [ ] Create `src/app/pricing/service_mappings.rs`
- [ ] Map CloudFormation resource types to AWS service codes
- [ ] Map CloudFormation parameters to pricing attributes
- [ ] Handle complex service relationships (EC2 + EBS, RDS + backup)
- [ ] Support region-specific pricing variations

**Mapping Examples:**
```rust
pub fn get_pricing_mappings() -> HashMap<&'static str, ServiceMapping> {
    let mut mappings = HashMap::new();
    
    mappings.insert("AWS::EC2::Instance", ServiceMapping {
        primary_service: "AmazonEC2",
        related_services: vec!["AmazonEBS"], // For EBS-optimized instances
        required_attributes: vec!["instanceType", "operatingSystem", "tenancy"],
        optional_attributes: vec!["preInstalledSw", "licenseModel"],
        parameter_mappings: HashMap::from([
            ("InstanceType", "instanceType"),
            ("KeyName", None), // No pricing impact
            ("SecurityGroups", None),
        ]),
    });
    
    mappings.insert("AWS::RDS::DBInstance", ServiceMapping {
        primary_service: "AmazonRDS",
        related_services: vec!["AmazonEBS"], // For storage
        required_attributes: vec!["databaseEngine", "instanceType", "deploymentOption"],
        optional_attributes: vec!["licenseModel"],
        parameter_mappings: HashMap::from([
            ("DBInstanceClass", "instanceType"),
            ("Engine", "databaseEngine"),
            ("MultiAZ", "deploymentOption"),
            ("StorageType", "storageMedia"),
        ]),
    });
}
```

#### 1.4 Regional Pricing Data Management
- [ ] Create `src/app/pricing/regional_pricing.rs`
- [ ] Support all AWS regions with accurate pricing
- [ ] Handle region-specific service availability
- [ ] Add currency conversion capabilities
- [ ] Support government/china region pricing differences

**Region Data Structure:**
```rust
pub struct RegionalPricing {
    region_code: String,
    region_name: String,
    available_services: HashSet<String>,
    pricing_data: HashMap<String, ServicePricing>,
    currency: String,
    last_updated: DateTime<Utc>,
}
```

---

## Milestone 2: CloudFormation Integration
**Goal**: Bridge CloudFormation parameters with pricing dimensions for accurate cost calculation

### Tasks

#### 2.1 CloudFormation Template Analysis
- [ ] Create `src/app/pricing/cfn_cost_analyzer.rs`
- [ ] Parse CloudFormation templates for cost-relevant resources
- [ ] Extract parameters that impact pricing
- [ ] Identify resource dependencies that affect costs
- [ ] Validate parameter values against pricing constraints

**Template Analysis Engine:**
```rust
pub struct CfnCostAnalyzer {
    pricing_client: Arc<AwsPricingClient>,
    service_mappings: ServiceMappings,
}

pub struct CostAnalysisResult {
    billable_resources: Vec<BillableResource>,
    pricing_parameters: Vec<PricingParameter>,
    missing_parameters: Vec<String>,
    warnings: Vec<String>,
}

pub struct BillableResource {
    logical_id: String,
    resource_type: String,
    primary_service: String,
    related_services: Vec<String>,
    pricing_attributes: HashMap<String, String>,
    depends_on: Vec<String>,
}
```

#### 2.2 Parameter-to-Pricing Validation
- [ ] Create `src/app/pricing/parameter_validator.rs`
- [ ] Validate CloudFormation parameter values against AWS pricing data
- [ ] Check instance type availability per region
- [ ] Validate storage types, database engines, etc.
- [ ] Provide suggestions for cost optimization

**Validation Rules:**
```rust
pub struct ParameterValidator {
    pricing_client: Arc<AwsPricingClient>,
}

pub struct ValidationResult {
    is_valid: bool,
    errors: Vec<ValidationError>,
    warnings: Vec<ValidationWarning>,
    suggestions: Vec<CostOptimizationSuggestion>,
}
```

#### 2.3 Infrastructure Cost Baseline Calculator
- [ ] Create `src/app/pricing/baseline_calculator.rs`
- [ ] Calculate infrastructure costs based on CloudFormation resources
- [ ] Support multiple pricing models (On-Demand, Reserved, Spot)
- [ ] Handle complex resource relationships
- [ ] Provide hourly, monthly, yearly projections

**Cost Calculation Engine:**
```rust
pub struct BaselineCalculator {
    pricing_client: Arc<AwsPricingClient>,
    regional_pricing: Arc<RegionalPricing>,
}

pub struct CostBaseline {
    total_hourly_cost: Decimal,
    total_monthly_cost: Decimal,
    total_yearly_cost: Decimal,
    cost_breakdown: HashMap<String, ServiceCost>,
    pricing_model_breakdown: HashMap<PricingModel, Decimal>,
}
```

---

## Milestone 3: Usage Scenario Modeling
**Goal**: Build comprehensive usage scenario framework for cost projections

### Tasks

#### 3.1 Usage Metrics Framework
- [ ] Create `src/app/pricing/usage_metrics.rs`
- [ ] Define usage patterns for all major AWS services
- [ ] Support time-based usage variations (daily, weekly, seasonal)
- [ ] Handle burst vs. steady-state workloads
- [ ] Include data transfer and storage growth patterns

**Usage Metrics Structure:**
```rust
pub struct UsageScenario {
    scenario_name: String,
    environment: String,
    infrastructure_params_ref: String,
    service_usage: HashMap<String, ServiceUsagePattern>,
    operational_assumptions: OperationalAssumptions,
    growth_projections: Option<GrowthProjections>,
    time_patterns: Option<TimeBasedUsage>,
}

pub struct ServiceUsagePattern {
    service_code: String,
    baseline_usage: HashMap<String, f64>, // metric_name -> value
    peak_multipliers: HashMap<String, f64>,
    seasonal_variations: Option<HashMap<String, f64>>,
}

pub struct TimeBasedUsage {
    hourly_patterns: Vec<f64>, // 24 hourly multipliers
    daily_patterns: Vec<f64>,  // 7 daily multipliers  
    monthly_patterns: Vec<f64>, // 12 monthly multipliers
}
```

#### 3.2 Scenario Template System
- [ ] Create `src/app/pricing/scenario_templates.rs`
- [ ] Build reusable usage pattern templates
- [ ] Support template inheritance and composition
- [ ] Include industry-specific templates (web apps, analytics, ML)
- [ ] Add template versioning and validation

**Template Categories:**
- Web Applications (light, medium, heavy traffic)
- Data Analytics (batch processing, real-time streaming)
- Machine Learning (training, inference, model serving)
- Enterprise Applications (CRM, ERP, collaboration)
- Development Workflows (CI/CD, testing, staging)

#### 3.3 Growth Projection Algorithms
- [ ] Create `src/app/pricing/growth_projections.rs`
- [ ] Linear, exponential, and S-curve growth models
- [ ] Seasonal adjustment algorithms
- [ ] Capacity planning with scaling thresholds
- [ ] Multi-year projection capabilities

#### 3.4 Operational Assumption Modeling
- [ ] Create `src/app/pricing/operational_assumptions.rs`
- [ ] Reserved Instance coverage optimization
- [ ] Spot Instance utilization patterns
- [ ] Savings Plans coverage analysis
- [ ] Right-sizing recommendations
- [ ] Data lifecycle management costs

---

## Milestone 4: Cost Analysis & Visualization
**Goal**: Provide comprehensive cost analysis interfaces and reporting

### Tasks

#### 4.1 Scenario Comparison Engine
- [ ] Create `src/app/pricing/scenario_comparison.rs`
- [ ] Side-by-side scenario cost comparisons
- [ ] What-if analysis capabilities
- [ ] Cost impact analysis for parameter changes
- [ ] Break-even analysis for Reserved Instances

#### 4.2 Cost Dashboard UI Components
- [ ] Create `src/app/dashui/pricing/cost_dashboard.rs`
- [ ] Real-time cost monitoring widgets
- [ ] Historical cost trend visualization
- [ ] Service-level cost breakdown charts
- [ ] Resource utilization vs. cost efficiency metrics

#### 4.3 Scenario Management UI
- [ ] Create `src/app/dashui/pricing/scenario_manager.rs`
- [ ] Scenario creation and editing interface
- [ ] Template application and customization
- [ ] Parameter sensitivity analysis visualization
- [ ] Export scenarios to Excel/CSV/PDF

#### 4.4 Cost Alerting System
- [ ] Create `src/app/pricing/cost_alerts.rs`
- [ ] Budget threshold monitoring
- [ ] Anomaly detection algorithms
- [ ] Cost spike notifications
- [ ] Projected budget overrun warnings

---

## Milestone 5: Advanced Features
**Goal**: Add sophisticated cost optimization and analysis capabilities

### Tasks

#### 5.1 Reserved Instance Optimization
- [ ] Create `src/app/pricing/ri_optimizer.rs`
- [ ] Analyze current usage patterns for RI recommendations
- [ ] Compare RI terms (1-year vs 3-year, partial vs full upfront)
- [ ] Portfolio-level RI optimization
- [ ] RI utilization tracking and optimization

#### 5.2 Spot Pricing Integration
- [ ] Create `src/app/pricing/spot_pricing.rs`
- [ ] Real-time spot price monitoring
- [ ] Spot instance interruption risk analysis
- [ ] Hybrid spot/on-demand cost modeling
- [ ] Spot fleet optimization recommendations

#### 5.3 Savings Plans Analysis
- [ ] Create `src/app/pricing/savings_plans.rs`
- [ ] Compute vs EC2 Savings Plans comparison
- [ ] Coverage analysis and optimization
- [ ] Multi-service Savings Plans modeling
- [ ] Break-even analysis for different commitment levels

#### 5.4 Multi-Region Cost Comparison
- [ ] Create `src/app/pricing/multi_region_analysis.rs`
- [ ] Cross-region pricing comparison
- [ ] Data transfer cost modeling
- [ ] Compliance and latency vs. cost trade-offs
- [ ] Disaster recovery cost analysis

---

## Integration Points with Current Codebase

### CloudFormation Manager Integration
```rust
// Add pricing estimation to CloudFormation Manager
impl CloudFormationManager {
    pub fn get_cost_estimator(&self) -> Option<Arc<CostEstimator>> {
        // Returns cost estimator using current project and parameters
    }
    
    pub async fn estimate_deployment_cost(
        &self,
        template: &str,
        parameters: &HashMap<String, String>,
        scenario: &UsageScenario,
    ) -> Result<CostEstimate> {
        // Integrates with parameter collection to provide real-time cost estimates
    }
}
```

### Project System Integration
```rust
// Extend Project struct for pricing data
pub struct Project {
    // ... existing fields ...
    pub pricing_scenarios: Vec<UsageScenario>,
    pub cost_estimates: HashMap<String, CostEstimate>, // scenario_name -> estimate
    pub cost_alerts: Vec<CostAlert>,
}
```

### Command Palette Integration
- Add "Estimate Costs" command to CloudFormation command palette
- "Compare Scenarios" command for cost analysis
- "Optimize Costs" command for recommendations

### AWS Explorer Integration
- Display real-time cost information for discovered resources
- Show cost trends for existing infrastructure
- Highlight cost optimization opportunities

## Technical Architecture

### Module Organization
```
src/app/pricing/
├── mod.rs                      # Module exports
├── aws_pricing_client.rs       # AWS API integration
├── pricing_cache.rs           # Caching system
├── service_mappings.rs        # CloudFormation to pricing mappings
├── regional_pricing.rs        # Regional pricing management
├── cfn_cost_analyzer.rs       # CloudFormation cost analysis
├── parameter_validator.rs     # Parameter pricing validation
├── baseline_calculator.rs     # Infrastructure cost calculation
├── usage_metrics.rs           # Usage pattern framework
├── scenario_templates.rs      # Reusable scenario templates
├── growth_projections.rs      # Growth modeling algorithms
├── operational_assumptions.rs # RI/Spot/Savings Plans modeling
├── scenario_comparison.rs     # Scenario analysis engine
├── cost_alerts.rs            # Cost monitoring and alerting
├── ri_optimizer.rs           # Reserved Instance optimization
├── spot_pricing.rs           # Spot instance analysis
├── savings_plans.rs          # Savings Plans analysis
└── multi_region_analysis.rs  # Cross-region cost comparison

src/app/dashui/pricing/
├── mod.rs                     # UI module exports  
├── cost_dashboard.rs          # Cost monitoring dashboard
├── scenario_manager.rs        # Scenario management UI
├── cost_comparison.rs         # Side-by-side comparisons
├── optimization_panel.rs      # Cost optimization recommendations
└── pricing_widgets.rs         # Reusable cost visualization components
```

### Data Persistence
- Extend project file structure with pricing data
- Separate files for scenarios per environment (matching parameter structure)
- Cached pricing data in `~/.local/share/awsdash/pricing_cache/`
- Scenario templates in application resources

### Performance Considerations
- Aggressive caching of pricing data (24-hour TTL)
- Lazy loading of regional pricing data
- Background pricing data updates
- Compressed storage for bulk pricing files
- Efficient diff algorithms for scenario comparisons

### Error Handling & Resilience
- Graceful degradation when pricing API unavailable
- Cached pricing fallback for offline scenarios
- Retry logic with exponential backoff
- User-friendly error messages for pricing API limits
- Validation warnings for stale pricing data

## Dependencies

### Additional Cargo Dependencies
```toml
# Pricing and financial calculations
rust_decimal = "1.32"          # Precise decimal arithmetic for pricing
chrono = { version = "0.4", features = ["serde"] } # Already included

# Data compression and caching
flate2 = "1.0"                 # Compression for bulk pricing data
sled = "0.34"                  # Embedded database for pricing cache

# Mathematical operations for growth projections
nalgebra = "0.32"              # Linear algebra for projection algorithms
```

### AWS SDK Requirements
- Existing `aws-sdk-pricing` (if available) or custom HTTP client
- No additional permissions required (pricing API is public)
- Optional: AWS Cost Explorer API for historical cost data

## Risk Mitigation

### High Risk Items
- AWS Pricing API rate limits during bulk operations
- Large pricing data files impacting application performance  
- Complex multi-service cost calculations accuracy
- Pricing data freshness and update coordination

### Mitigation Strategies
- Implement progressive loading and background updates
- Comprehensive caching with intelligent invalidation
- Extensive unit testing with real pricing data samples
- User feedback loops for pricing estimate accuracy validation
- Graceful degradation for pricing service outages

## Success Criteria

### Milestone 1 Success
- Successfully cache and query AWS pricing data for all major services
- Handle regional pricing variations accurately
- Provide sub-second pricing lookups for common scenarios

### Milestone 2 Success  
- Accurately map CloudFormation parameters to pricing dimensions
- Provide infrastructure cost estimates within 5% of actual AWS billing
- Validate all parameter values against current AWS pricing

### Milestone 3 Success
- Support comprehensive usage scenario modeling
- Enable cost projections across multiple time horizons
- Provide template-based scenario creation

### Milestone 4 Success
- Deliver intuitive cost analysis and comparison interfaces
- Enable scenario-based cost planning workflows
- Provide actionable cost optimization recommendations

### Final Success
- Production-ready cost estimation and scenario planning
- Significantly improved cost visibility compared to AWS Console
- Seamless integration with existing AWS Dash CloudFormation workflows

## Research Resources for Future Reference

### AWS Pricing API Documentation
- [AWS Price List Query API](https://docs.aws.amazon.com/awsaccountbilling/latest/aboutv2/using-price-list-query-api.html)
- [GetProducts API Reference](https://docs.aws.amazon.com/aws-cost-management/latest/APIReference/API_pricing_GetProducts.html)
- [AWS Pricing Calculator](https://calculator.aws/)
- [AWS Price List API Blog](https://aws.amazon.com/blogs/aws/aws-price-list-api-update-new-query-and-metadata-functions/)

### Third-Party Resources
- [PilotCore AWS Pricing API Guide](https://pilotcore.io/blog/how-to-use-aws-price-list-api-examples)
- [CloudQuery AWS Pricing Plugin](https://www.cloudquery.io/blog/exploring-aws-pricing-api)
- [Virtana AWS Pricing API Analysis](https://www.virtana.com/blog/aws-pricing-api/)

### Key API Endpoints for Testing
- Bulk API: `https://pricing.us-east-1.amazonaws.com`
- Query API: `https://api.pricing.us-east-1.amazonaws.com`
- EC2 Pricing: `https://pricing.us-east-1.amazonaws.com/offers/v1.0/aws/AmazonEC2/current/us-east-1/index.json`

### Service-Specific Pricing Complexity Examples
- **EC2**: 20+ attributes including instanceType, operatingSystem, tenancy, preInstalledSw
- **RDS**: Database engine, deployment option, license model, storage type
- **Lambda**: Architecture, memory, request count, duration  
- **S3**: Storage class, request type, data transfer, management features

### Implementation Timeline Estimate
- **Milestone 1-2**: 6-8 weeks (Core infrastructure + CloudFormation integration)
- **Milestone 3**: 4-6 weeks (Usage scenario modeling)
- **Milestone 4**: 4-6 weeks (UI and visualization)
- **Milestone 5**: 6-8 weeks (Advanced optimization features)
- **Total**: 20-28 weeks for complete implementation

---

*This epic provides the foundation for sophisticated AWS cost management within AWS Dash, complementing the CloudFormation Manager with comprehensive pricing intelligence and scenario planning capabilities.*