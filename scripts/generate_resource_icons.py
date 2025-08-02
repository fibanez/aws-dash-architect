#!/usr/bin/env python3
"""
Generate cfn_resource_icons.rs by matching CloudFormation resources with AWS icons.

This script:
1. Downloads CloudFormation resource specifications from AWS
2. Searches for matching icons in the assets/Icons folder
3. Generates a Rust file with resource-to-icon mappings

AWS Architecture Icons are used under AWS's permitted usage terms for customers
creating architecture diagrams. Icons are provided by Amazon Web Services, Inc.
Source: https://aws.amazon.com/architecture/icons/
"""

import json
import os
import re
import urllib.request
import gzip
from pathlib import Path
from typing import Dict, List, Optional, Tuple
from collections import defaultdict

# Configuration
HOME = Path.home()
RESOURCE_SPEC_DIR = HOME / ".config" / "awsdash" / "cfn-resources"
RESOURCE_SPEC_FILE = RESOURCE_SPEC_DIR / "us-east-1.json"
RESOURCE_SPEC_URL = "https://d1uauaxba7bl26.cloudfront.net/latest/gzip/CloudFormationResourceSpecification.json"

# Icon directories (relative to the project root)
ICONS_BASE_DIR = Path("assets/Icons")
ARCH_SERVICE_ICONS_PATTERN = "Architecture-Service-Icons_*"
RESOURCE_ICONS_PATTERN = "Resource-Icons_*"
ARCH_GROUP_ICONS_PATTERN = "Architecture-Group-Icons_*"

# Output file
OUTPUT_FILE = Path("src/app/cfn_resource_icons.rs")

# Icon size preference order
ICON_SIZE_PREFERENCE = ["16", "32", "48", "64"]

def download_resource_spec() -> dict:
    """Download CloudFormation resource specification if not cached."""
    if RESOURCE_SPEC_FILE.exists():
        print(f"Using cached resource specification from {RESOURCE_SPEC_FILE}")
        with open(RESOURCE_SPEC_FILE, 'r') as f:
            return json.load(f)
    
    print(f"Downloading CloudFormation resource specification from AWS...")
    RESOURCE_SPEC_DIR.mkdir(parents=True, exist_ok=True)
    
    # Download as gzipped file first
    gz_file = RESOURCE_SPEC_FILE.with_suffix('.json.gz')
    urllib.request.urlretrieve(RESOURCE_SPEC_URL, gz_file)
    
    # Decompress
    with gzip.open(gz_file, 'rb') as f_in:
        with open(RESOURCE_SPEC_FILE, 'wb') as f_out:
            f_out.write(f_in.read())
    
    # Remove gz file
    gz_file.unlink()
    print(f"Downloaded and extracted to {RESOURCE_SPEC_FILE}")
    
    with open(RESOURCE_SPEC_FILE, 'r') as f:
        return json.load(f)

def get_all_resource_types(spec: dict) -> List[str]:
    """Extract all resource types from the specification."""
    resource_types = list(spec.get("ResourceTypes", {}).keys())
    
    # Also include property types as some might be referenced
    property_types = list(spec.get("PropertyTypes", {}).keys())
    
    # Filter property types to get unique service resources
    for prop_type in property_types:
        parts = prop_type.split(".")
        if len(parts) >= 2:
            base_resource = "::".join(parts[0].split("::")[:-1] + [parts[0].split("::")[-1]])
            if base_resource not in resource_types:
                resource_types.append(base_resource)
    
    return sorted(set(resource_types))

def normalize_service_name(service: str) -> List[str]:
    """Generate possible icon name variations for a service."""
    variations = []
    
    # Original name
    variations.append(service)
    
    # Handle special cases
    service_mappings = {
        "EC2": ["EC2", "Elastic-Compute-Cloud", "Amazon-EC2"],
        "S3": ["S3", "Simple-Storage-Service", "Amazon-Simple-Storage-Service"],
        "RDS": ["RDS", "Amazon-RDS", "Relational-Database-Service"],
        "Lambda": ["Lambda", "AWS-Lambda"],
        "DynamoDB": ["DynamoDB", "Amazon-DynamoDB"],
        "CloudFormation": ["CloudFormation", "AWS-CloudFormation"],
        "CloudWatch": ["CloudWatch", "Amazon-CloudWatch"],
        "CloudTrail": ["CloudTrail", "AWS-CloudTrail"],
        "SNS": ["SNS", "Simple-Notification-Service", "Amazon-Simple-Notification-Service"],
        "SQS": ["SQS", "Simple-Queue-Service", "Amazon-Simple-Queue-Service"],
        "IAM": ["IAM", "Identity-and-Access-Management", "AWS-Identity-and-Access-Management"],
        "KMS": ["KMS", "Key-Management-Service", "AWS-Key-Management-Service"],
        "ECS": ["ECS", "Elastic-Container-Service", "Amazon-Elastic-Container-Service"],
        "EKS": ["EKS", "Elastic-Kubernetes-Service", "Amazon-Elastic-Kubernetes-Service"],
        "ECR": ["ECR", "Elastic-Container-Registry", "Amazon-Elastic-Container-Registry"],
        "ElastiCache": ["ElastiCache", "Amazon-ElastiCache"],
        "ElasticLoadBalancing": ["Elastic-Load-Balancing", "ELB"],
        "ElasticLoadBalancingV2": ["Elastic-Load-Balancing", "ELB"],
        "AutoScaling": ["Auto-Scaling", "EC2-Auto-Scaling", "Amazon-EC2-Auto-Scaling"],
        "ApiGateway": ["API-Gateway", "Amazon-API-Gateway"],
        "ApiGatewayV2": ["API-Gateway", "Amazon-API-Gateway"],
        "Cognito": ["Cognito", "Amazon-Cognito"],
        "SecretsManager": ["Secrets-Manager", "AWS-Secrets-Manager"],
        "EventBridge": ["EventBridge", "Amazon-EventBridge"],
        "Events": ["EventBridge", "Amazon-EventBridge"],
        "StepFunctions": ["Step-Functions", "AWS-Step-Functions"],
        "Glue": ["Glue", "AWS-Glue"],
        "SageMaker": ["SageMaker", "Amazon-SageMaker", "Amazon-SageMaker-AI"],
        "CodePipeline": ["CodePipeline", "AWS-CodePipeline"],
        "CodeBuild": ["CodeBuild", "AWS-CodeBuild"],
        "CodeCommit": ["CodeCommit", "AWS-CodeCommit"],
        "CodeDeploy": ["CodeDeploy", "AWS-CodeDeploy"],
        "Kinesis": ["Kinesis", "Amazon-Kinesis"],
        "KinesisFirehose": ["Kinesis", "Data-Firehose", "Amazon-Data-Firehose"],
        "KinesisAnalytics": ["Kinesis", "Managed-Service-for-Apache-Flink"],
        "Athena": ["Athena", "Amazon-Athena"],
        "Redshift": ["Redshift", "Amazon-Redshift"],
        "EFS": ["EFS", "Elastic-File-System", "Amazon-Elastic-File-System"],
        "Backup": ["Backup", "AWS-Backup"],
        "WAF": ["WAF", "AWS-WAF"],
        "WAFv2": ["WAF", "AWS-WAF"],
        "Config": ["Config", "AWS-Config"],
        "SSM": ["Systems-Manager", "AWS-Systems-Manager"],
        "AppSync": ["AppSync", "AWS-AppSync"],
        "Amplify": ["Amplify", "AWS-Amplify"],
        "ElasticBeanstalk": ["Elastic-Beanstalk", "AWS-Elastic-Beanstalk"],
        "OpenSearchService": ["OpenSearch-Service", "Amazon-OpenSearch-Service"],
        "IoT": ["IoT", "AWS-IoT", "AWS-IoT-Core"],
        "MSK": ["MSK", "Managed-Streaming-for-Apache-Kafka", "Amazon-Managed-Streaming-for-Apache-Kafka"],
        "DocDB": ["DocumentDB", "Amazon-DocumentDB"],
        "Neptune": ["Neptune", "Amazon-Neptune"],
        "QLDB": ["QLDB", "Quantum-Ledger-Database", "Amazon-Quantum-Ledger-Database"],
        "Timestream": ["Timestream", "Amazon-Timestream"],
        "Route53": ["Route-53", "Amazon-Route-53"],
        "CloudFront": ["CloudFront", "Amazon-CloudFront"],
        "ACM": ["Certificate-Manager", "AWS-Certificate-Manager"],
        "AppFlow": ["AppFlow", "Amazon-AppFlow"],
        "AppConfig": ["AppConfig", "AWS-AppConfig"],
        "Batch": ["Batch", "AWS-Batch"],
        "DataSync": ["DataSync", "AWS-DataSync"],
        "DMS": ["Database-Migration-Service", "AWS-Database-Migration-Service"],
        "EMR": ["EMR", "Amazon-EMR"],
        "FSx": ["FSx", "Amazon-FSx"],
        "GameLift": ["GameLift", "Amazon-GameLift"],
        "Macie": ["Macie", "Amazon-Macie"],
        "MQ": ["MQ", "Amazon-MQ"],
        "QuickSight": ["QuickSight", "Amazon-QuickSight"],
        "WorkSpaces": ["WorkSpaces", "Amazon-WorkSpaces"],
        "LakeFormation": ["Lake-Formation", "AWS-Lake-Formation"],
        "DataExchange": ["Data-Exchange", "AWS-Data-Exchange"],
        "FinSpace": ["FinSpace", "Amazon-FinSpace"],
        "Forecast": ["Forecast", "Amazon-Forecast"],
        "Comprehend": ["Comprehend", "Amazon-Comprehend"],
        "Translate": ["Translate", "Amazon-Translate"],
        "Transcribe": ["Transcribe", "Amazon-Transcribe"],
        "Rekognition": ["Rekognition", "Amazon-Rekognition"],
        "Textract": ["Textract", "Amazon-Textract"],
        "Polly": ["Polly", "Amazon-Polly"],
        "Lex": ["Lex", "Amazon-Lex"],
        "Connect": ["Connect", "Amazon-Connect"],
        "Pinpoint": ["Pinpoint", "Amazon-Pinpoint"],
        "SES": ["SES", "Simple-Email-Service", "Amazon-Simple-Email-Service"],
        "Chime": ["Chime", "Amazon-Chime"],
        "WorkMail": ["WorkMail", "Amazon-WorkMail"],
        "Shield": ["Shield", "AWS-Shield"],
        "GuardDuty": ["GuardDuty", "Amazon-GuardDuty"],
        "Inspector": ["Inspector", "Amazon-Inspector"],
        "SecurityHub": ["Security-Hub", "AWS-Security-Hub"],
        "Artifact": ["Artifact", "AWS-Artifact"],
        "Audit": ["Audit-Manager", "AWS-Audit-Manager"],
        "ControlTower": ["Control-Tower", "AWS-Control-Tower"],
        "Organizations": ["Organizations", "AWS-Organizations"],
        "ResourceGroups": ["Resource-Groups", "AWS-Resource-Groups"],
        "ServiceCatalog": ["Service-Catalog", "AWS-Service-Catalog"],
        "CloudMap": ["Cloud-Map", "AWS-Cloud-Map"],
        "AppMesh": ["App-Mesh", "AWS-App-Mesh"],
        "XRay": ["X-Ray", "AWS-X-Ray"],
        "DevOpsGuru": ["DevOps-Guru", "Amazon-DevOps-Guru"],
    }
    
    # Remove AWS:: prefix if present
    if service.startswith("AWS::"):
        service = service[5:]
    
    # Check if we have specific mappings
    if service in service_mappings:
        variations.extend(service_mappings[service])
    
    # Generate hyphenated version
    hyphenated = re.sub(r'([a-z])([A-Z])', r'\1-\2', service)
    variations.append(hyphenated)
    
    # Add AWS- and Amazon- prefixes
    for var in list(variations):
        variations.append(f"AWS-{var}")
        variations.append(f"Amazon-{var}")
    
    return list(set(variations))

def find_icon_for_resource(resource_type: str, icons_base: Path) -> Optional[str]:
    """Find the best matching icon for a CloudFormation resource type."""
    # Parse resource type
    parts = resource_type.split("::")
    if len(parts) < 3:
        return None
    
    provider = parts[0]  # AWS
    service = parts[1]   # e.g., EC2, S3, Lambda
    resource = parts[2]  # e.g., Instance, Bucket, Function
    
    # Generate service name variations
    service_variations = normalize_service_name(service)
    
    # Helper function to get relative path
    def get_relative_path(icon_file: Path) -> str:
        """Get relative path from project root."""
        # Convert to absolute paths for comparison
        icon_abs = icon_file.resolve()
        cwd_abs = Path.cwd().resolve()
        icons_base_abs = icons_base.resolve()
        
        # If icon is under current directory
        try:
            return str(icon_abs.relative_to(cwd_abs))
        except ValueError:
            # Otherwise construct path from icons base
            try:
                rel_from_icons = icon_abs.relative_to(icons_base_abs.parent.parent)
                return str(rel_from_icons)
            except ValueError:
                # Last resort: return the original path
                return str(icon_file)
    
    # Search order:
    # 1. Architecture-Service-Icons (preferred)
    # 2. Resource-Icons
    # 3. Architecture-Group-Icons
    
    # First, try Architecture-Service-Icons
    for arch_dir in sorted(icons_base.glob(ARCH_SERVICE_ICONS_PATTERN)):
        for size in ICON_SIZE_PREFERENCE:
            for service_var in service_variations:
                # Try different naming patterns
                patterns = [
                    f"Arch_{service_var}_{size}.png",
                    f"Arch_{service_var}_{size}.svg",
                    f"Arch-{service_var}_{size}.png",
                    f"Arch-{service_var}_{size}.svg",
                ]
                
                for pattern in patterns:
                    # Search in category subdirectories
                    for category_dir in arch_dir.iterdir():
                        if category_dir.is_dir():
                            size_dir = category_dir / size
                            if size_dir.exists():
                                for icon_file in size_dir.glob(pattern):
                                    if icon_file.exists():
                                        return get_relative_path(icon_file)
    
    # Second, try Resource-Icons with more specific matching
    for res_dir in sorted(icons_base.glob(RESOURCE_ICONS_PATTERN)):
        for service_var in service_variations:
            # Try to match based on resource type too
            resource_patterns = [
                f"*{service_var}*{resource}*.png",
                f"*{service_var}*{resource}*.svg",
                f"*{service_var}*.png",
                f"*{service_var}*.svg",
            ]
            
            for pattern in resource_patterns:
                for icon_file in res_dir.rglob(pattern):
                    if icon_file.exists() and "48" in str(icon_file):  # Prefer 48px for resource icons
                        return get_relative_path(icon_file)
    
    # Third, try Architecture-Group-Icons
    for group_dir in sorted(icons_base.glob(ARCH_GROUP_ICONS_PATTERN)):
        for service_var in service_variations:
            patterns = [
                f"*{service_var}*.png",
                f"*{service_var}*.svg",
            ]
            
            for pattern in patterns:
                for icon_file in group_dir.glob(pattern):
                    if icon_file.exists() and "32" in str(icon_file):  # Prefer 32px for group icons
                        return get_relative_path(icon_file)
    
    return None

def generate_rust_file(resource_icon_map: Dict[str, str], output_file: Path):
    """Generate the Rust source file with resource-to-icon mappings."""
    rust_content = '''use std::collections::HashMap;
use once_cell::sync::Lazy;
use tracing::{debug, warn};

/// Map of CloudFormation resource types to their corresponding icon paths
/// Icons are from the AWS Architecture Icons pack
pub static RESOURCE_ICONS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut map = HashMap::new();
    
'''
    
    # Group resources by service for better organization
    service_groups = defaultdict(list)
    for resource_type, icon_path in sorted(resource_icon_map.items()):
        parts = resource_type.split("::")
        if len(parts) >= 2:
            service = parts[1]
            service_groups[service].append((resource_type, icon_path))
        else:
            service_groups["Other"].append((resource_type, icon_path))
    
    # Write entries grouped by service
    for service in sorted(service_groups.keys()):
        rust_content += f"    // {service} Resources\n"
        for resource_type, icon_path in service_groups[service]:
            rust_content += f'    map.insert("{resource_type}", "{icon_path}");\n'
        rust_content += "\n"
    
    # Add default icon
    rust_content += '''    // Default icon for unknown resource types
    map.insert("default", "assets/Icons/Architecture-Group-Icons_02072025/AWS-Cloud_32.png");
    
    map
});

/// Get the icon path for a given CloudFormation resource type
pub fn get_icon_for_resource(resource_type: &str) -> &'static str {
    if let Some(icon_path) = RESOURCE_ICONS.get(resource_type) {
        debug!("Found exact icon match for resource type: {} -> {}", resource_type, icon_path);
        return icon_path;
    }
    
    // Try to match by service prefix if exact match not found
    let service_prefix = resource_type.split("::").take(2).collect::<Vec<_>>().join("::");
    debug!("No exact match for {}, trying service prefix: {}", resource_type, service_prefix);
    
    // Check if we have any resources that start with this service prefix
    for (key, value) in RESOURCE_ICONS.iter() {
        if key.starts_with(&service_prefix) && *key != "default" {
            debug!("Found service prefix match: {} -> {}", key, value);
            return value;
        }
    }
    
    // Return default icon if no match found
    let default_icon = RESOURCE_ICONS.get("default").unwrap();
    warn!("No icon found for resource type: {}, using default: {}", resource_type, default_icon);
    default_icon
}
'''
    
    output_file.parent.mkdir(parents=True, exist_ok=True)
    with open(output_file, 'w') as f:
        f.write(rust_content)
    
    print(f"Generated {output_file} with {len(resource_icon_map)} resource mappings")

def main():
    """Main function to generate the resource icons mapping."""
    # Download or load resource specification
    spec = download_resource_spec()
    
    # Get all resource types
    resource_types = get_all_resource_types(spec)
    print(f"Found {len(resource_types)} CloudFormation resource types")
    
    # Find icons for each resource
    resource_icon_map = {}
    icons_found = 0
    
    for resource_type in resource_types:
        icon_path = find_icon_for_resource(resource_type, ICONS_BASE_DIR)
        if icon_path:
            resource_icon_map[resource_type] = icon_path
            icons_found += 1
            print(f"✓ {resource_type} -> {icon_path}")
        else:
            print(f"✗ {resource_type} -> No icon found")
    
    print(f"\nFound icons for {icons_found}/{len(resource_types)} resources ({icons_found/len(resource_types)*100:.1f}%)")
    
    # Generate Rust file
    generate_rust_file(resource_icon_map, OUTPUT_FILE)
    
    print(f"\nDone! Generated {OUTPUT_FILE}")

if __name__ == "__main__":
    main()