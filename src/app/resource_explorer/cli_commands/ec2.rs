//! EC2 service CLI commands and field mappings.
//!
//! Covers: Instance, SecurityGroup, VPC, Subnet, Volume

use super::{CliCommand, ComparisonType, FieldMapping};

// ============================================================================
// EC2 Instance
// ============================================================================

pub fn instance_cli_command() -> CliCommand {
    CliCommand {
        service: "ec2",
        operation: "describe-instances",
        json_path: "Reservations[].Instances[]",
        id_field: "InstanceId",
        is_global: false,
        extra_args: &[],
    }
}

pub fn instance_field_mappings() -> Vec<FieldMapping> {
    vec![
        FieldMapping {
            dash_field: "InstanceId",
            cli_field: "InstanceId",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "InstanceType",
            cli_field: "InstanceType",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "ImageId",
            cli_field: "ImageId",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "State",
            cli_field: "State.Name",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "PrivateIpAddress",
            cli_field: "PrivateIpAddress",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "PublicIpAddress",
            cli_field: "PublicIpAddress",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "VpcId",
            cli_field: "VpcId",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "SubnetId",
            cli_field: "SubnetId",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Architecture",
            cli_field: "Architecture",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Platform",
            cli_field: "Platform",
            comparison_type: ComparisonType::Exact,
        },
    ]
}

// ============================================================================
// EC2 Security Group
// ============================================================================

pub fn security_group_cli_command() -> CliCommand {
    CliCommand {
        service: "ec2",
        operation: "describe-security-groups",
        json_path: "SecurityGroups",
        id_field: "GroupId",
        is_global: false,
        extra_args: &[],
    }
}

pub fn security_group_field_mappings() -> Vec<FieldMapping> {
    vec![
        FieldMapping {
            dash_field: "GroupId",
            cli_field: "GroupId",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "GroupName",
            cli_field: "GroupName",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Description",
            cli_field: "Description",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "VpcId",
            cli_field: "VpcId",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "OwnerId",
            cli_field: "OwnerId",
            comparison_type: ComparisonType::Exact,
        },
        // Complex fields - ignore for now as they require deep array comparison
        FieldMapping {
            dash_field: "IpPermissions",
            cli_field: "IpPermissions",
            comparison_type: ComparisonType::Ignore,
        },
        FieldMapping {
            dash_field: "IpPermissionsEgress",
            cli_field: "IpPermissionsEgress",
            comparison_type: ComparisonType::Ignore,
        },
    ]
}

// ============================================================================
// EC2 VPC
// ============================================================================

pub fn vpc_cli_command() -> CliCommand {
    CliCommand {
        service: "ec2",
        operation: "describe-vpcs",
        json_path: "Vpcs",
        id_field: "VpcId",
        is_global: false,
        extra_args: &[],
    }
}

pub fn vpc_field_mappings() -> Vec<FieldMapping> {
    vec![
        FieldMapping {
            dash_field: "VpcId",
            cli_field: "VpcId",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "CidrBlock",
            cli_field: "CidrBlock",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "State",
            cli_field: "State",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "IsDefault",
            cli_field: "IsDefault",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "InstanceTenancy",
            cli_field: "InstanceTenancy",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "DhcpOptionsId",
            cli_field: "DhcpOptionsId",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "OwnerId",
            cli_field: "OwnerId",
            comparison_type: ComparisonType::Exact,
        },
    ]
}

// ============================================================================
// EC2 Subnet
// ============================================================================

pub fn subnet_cli_command() -> CliCommand {
    CliCommand {
        service: "ec2",
        operation: "describe-subnets",
        json_path: "Subnets",
        id_field: "SubnetId",
        is_global: false,
        extra_args: &[],
    }
}

pub fn subnet_field_mappings() -> Vec<FieldMapping> {
    vec![
        FieldMapping {
            dash_field: "SubnetId",
            cli_field: "SubnetId",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "VpcId",
            cli_field: "VpcId",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "CidrBlock",
            cli_field: "CidrBlock",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "AvailabilityZone",
            cli_field: "AvailabilityZone",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "State",
            cli_field: "State",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "AvailableIpAddressCount",
            cli_field: "AvailableIpAddressCount",
            comparison_type: ComparisonType::Numeric,
        },
        FieldMapping {
            dash_field: "DefaultForAz",
            cli_field: "DefaultForAz",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "MapPublicIpOnLaunch",
            cli_field: "MapPublicIpOnLaunch",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "OwnerId",
            cli_field: "OwnerId",
            comparison_type: ComparisonType::Exact,
        },
    ]
}

// ============================================================================
// EC2 Volume
// ============================================================================

pub fn volume_cli_command() -> CliCommand {
    CliCommand {
        service: "ec2",
        operation: "describe-volumes",
        json_path: "Volumes",
        id_field: "VolumeId",
        is_global: false,
        extra_args: &[],
    }
}

pub fn volume_field_mappings() -> Vec<FieldMapping> {
    vec![
        FieldMapping {
            dash_field: "VolumeId",
            cli_field: "VolumeId",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Size",
            cli_field: "Size",
            comparison_type: ComparisonType::Numeric,
        },
        FieldMapping {
            dash_field: "VolumeType",
            cli_field: "VolumeType",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "State",
            cli_field: "State",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "AvailabilityZone",
            cli_field: "AvailabilityZone",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Encrypted",
            cli_field: "Encrypted",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Iops",
            cli_field: "Iops",
            comparison_type: ComparisonType::Numeric,
        },
        FieldMapping {
            dash_field: "SnapshotId",
            cli_field: "SnapshotId",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "MultiAttachEnabled",
            cli_field: "MultiAttachEnabled",
            comparison_type: ComparisonType::Exact,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instance_cli_command() {
        let cmd = instance_cli_command();
        assert_eq!(cmd.service, "ec2");
        assert_eq!(cmd.operation, "describe-instances");
        assert!(!cmd.is_global);
    }

    #[test]
    fn test_security_group_field_mappings() {
        let mappings = security_group_field_mappings();
        assert!(mappings.iter().any(|m| m.dash_field == "GroupId"));
        assert!(mappings.iter().any(|m| m.dash_field == "OwnerId"));
    }

    #[test]
    fn test_vpc_field_mappings() {
        let mappings = vpc_field_mappings();
        assert!(mappings.iter().any(|m| m.dash_field == "VpcId"));
        assert!(mappings.iter().any(|m| m.dash_field == "CidrBlock"));
    }

    #[test]
    fn test_subnet_cli_command() {
        let cmd = subnet_cli_command();
        assert_eq!(cmd.service, "ec2");
        assert_eq!(cmd.operation, "describe-subnets");
        assert_eq!(cmd.id_field, "SubnetId");
        assert!(!cmd.is_global);
    }

    #[test]
    fn test_subnet_field_mappings() {
        let mappings = subnet_field_mappings();
        assert!(mappings.iter().any(|m| m.dash_field == "SubnetId"));
        assert!(mappings.iter().any(|m| m.dash_field == "VpcId"));
        assert!(mappings.iter().any(|m| m.dash_field == "CidrBlock"));
        assert!(mappings.iter().any(|m| m.dash_field == "AvailabilityZone"));
    }

    #[test]
    fn test_volume_cli_command() {
        let cmd = volume_cli_command();
        assert_eq!(cmd.service, "ec2");
        assert_eq!(cmd.operation, "describe-volumes");
        assert_eq!(cmd.id_field, "VolumeId");
        assert!(!cmd.is_global);
    }

    #[test]
    fn test_volume_field_mappings() {
        let mappings = volume_field_mappings();
        assert!(mappings.iter().any(|m| m.dash_field == "VolumeId"));
        assert!(mappings.iter().any(|m| m.dash_field == "Size"));
        assert!(mappings.iter().any(|m| m.dash_field == "VolumeType"));
        assert!(mappings.iter().any(|m| m.dash_field == "Encrypted"));
    }
}
