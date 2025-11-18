---
name: aws-ec2-troubleshooting
description: Diagnose and resolve EC2 instance issues including networking, storage, performance, and connectivity
---

# AWS EC2 Troubleshooting Skill

## Purpose
This skill provides systematic diagnostic procedures for EC2 instance issues. Use semantic understanding to recognize when a user's problem matches this skill - look for availability, connectivity, performance, or operational issues with EC2 instances.

## When to Use This Skill
- Instance not responding to SSH/RDP
- Instance showing unexpected behavior
- Performance degradation or high resource utilization
- Networking connectivity issues
- Storage/EBS problems
- Instance status checks failing
- Console output errors

## Diagnostic Workflow

### 1. Gather Instance Information
Before investigating, collect:
- **Account ID**: Use `aws_find_account` if not specified
- **Region**: Use `aws_find_region` if not specified
- **Instance ID**: Ask user if not provided
- **Instance type**: Get from describe_resource
- **VPC ID / Subnet ID**: Get from describe_resource

**Tools to use**:
```
aws_find_account(filter="production")
aws_find_region(filter="us-east")
aws_describe_resource(resource_type="EC2::Instance", resource_id="{instance-id}")
```

### 2. Check Instance Status
Create task agent to verify:
- **Instance state**: running, stopped, terminated, stopping
- **Status checks**: System status (AWS infrastructure), Instance status (OS/application)
- **Console output**: Boot errors, kernel panics
- **System logs**: Check for kernel issues, network errors

**Task delegation**:
```
create_task(
  "Check EC2 instance {instance-id} status including state, status checks, and console output.
   Output: Instance state, system status check, instance status check, recent console output.
   Tools: aws_describe_resource(EC2::Instance).
   Boundaries: Focus on instance health, not configuration details.",
  account_id, region
)
```

### 3. Network Diagnostics
If connectivity issues suspected, create task agent to verify:
- **Security group rules**: Inbound/outbound rules for required ports (22, 3389, 80, 443, etc.)
- **Network ACLs**: Verify subnet NACL allows traffic
- **Route tables**: Verify routes to internet gateway or NAT gateway
- **Elastic IP/Public IP**: Check IP assignment
- **DNS resolution**: Verify hostname resolves
- **VPC configuration**: Ensure VPC has internet access

**Task delegation**:
```
create_task(
  "Diagnose network connectivity for EC2 instance {instance-id}.
   Output: Table with security group rules (inbound/outbound), NACL rules, route table entries, IP assignments.
   Tools: aws_describe_resource(EC2::SecurityGroup, EC2::NetworkAcl, EC2::RouteTable).
   Boundaries: Only check resources associated with instance, ignore unrelated VPC resources.",
  account_id, region
)
```

### 4. Storage Diagnostics
For storage-related issues:
- **EBS volume status**: Check volume state (available, in-use, error)
- **Volume attachments**: Verify volumes attached correctly
- **Disk space utilization**: Check CloudWatch metrics
- **I/O performance**: Review IOPS and throughput metrics
- **Volume type**: Confirm volume type matches workload (gp3, io2, etc.)

**Task delegation**:
```
create_task(
  "Check EBS volumes for EC2 instance {instance-id}.
   Output: Table with volume IDs, states, sizes, types, attachment status, and CloudWatch disk metrics.
   Tools: aws_describe_resource(EC2::Volume), aws_get_log_events (CloudWatch metrics).
   Boundaries: Only volumes attached to this instance.",
  account_id, region
)
```

### 5. Performance Analysis
For performance degradation:
- **CPU utilization**: Check CloudWatch CPUUtilization metric
- **Memory pressure**: Check CloudWatch Agent memory metrics if available
- **Network throughput**: Review NetworkIn/NetworkOut
- **EBS I/O metrics**: Check VolumeReadOps, VolumeWriteOps
- **Instance type appropriateness**: Compare utilization to instance limits

**Task delegation**:
```
create_task(
  "Analyze performance metrics for EC2 instance {instance-id} over last 24 hours.
   Output: Time-series data for CPU, memory, network, disk I/O with peak values and averages.
   Tools: aws_get_log_events (CloudWatch metrics).
   Boundaries: Last 24 hours only, focus on resource utilization metrics.",
  account_id, region
)
```

## Task Agent Orchestration

**Sequential execution** for dependencies:
1. Gather instance info (blocking - need instance details)
2. Check instance status (blocking - determines if running)
3. Run diagnostics in parallel (non-blocking):
   - Network diagnostics
   - Storage diagnostics
   - Performance analysis

**Parallel execution example**:
```
# After confirming instance is running, run these in parallel:
create_task("Network diagnostics for {instance-id}", account, region)
create_task("Storage diagnostics for {instance-id}", account, region)
create_task("Performance analysis for {instance-id}", account, region)
```

## Expected Outputs

Each diagnostic should return:
- **Status**: OK / WARNING / CRITICAL
- **Findings**: List of issues discovered
- **Recommendations**: Specific remediation steps

Example output format:
```
Network Diagnostics: WARNING
Findings:
- Security group sg-123abc allows 0.0.0.0/0 on port 22 (SSH)
- No outbound internet route in route table rtb-456def
Recommendations:
- Restrict SSH to specific IP ranges
- Add route 0.0.0.0/0 → igw-789ghi to route table
```

## Common Issues and Solutions

### Issue: Cannot connect via SSH
**Diagnostic checklist**:
1. Instance state = running? (If not, start instance)
2. Security group allows port 22 from your IP? (If not, add inbound rule)
3. Network ACL allows inbound 22 and outbound ephemeral ports? (If not, update NACL)
4. Route table has internet gateway route (0.0.0.0/0 → igw-xxx)? (If not, add route)
5. SSH key is correct? (Verify you're using the right key pair)
6. SSH service running on instance? (Check system logs)

### Issue: Instance status check failed (system status OK)
**Diagnostic checklist**:
1. Check console output for boot errors
2. Review system logs for kernel panics, out-of-memory errors
3. Verify EBS volumes are healthy and attached
4. Check for filesystem corruption (may need to stop instance and attach volume to another instance)
5. Review CloudWatch logs for application errors

### Issue: Instance status check failed (system status failed)
**Diagnostic checklist**:
1. System status failures indicate AWS infrastructure issues
2. Check AWS Service Health Dashboard for region issues
3. Stop and start instance (not reboot) to move to new hardware
4. Contact AWS support if issue persists

### Issue: High CPU utilization
**Diagnostic checklist**:
1. Identify process causing high CPU (requires SSH access or CloudWatch agent)
2. Review recent deployments or code changes
3. Check if instance type is appropriate for workload
4. Consider vertical scaling (larger instance type) or horizontal scaling (more instances)
5. Review application logs for errors or infinite loops

### Issue: Cannot reach internet from instance
**Diagnostic checklist**:
1. Instance in public subnet? Check route table for 0.0.0.0/0 → igw-xxx
2. Instance in private subnet? Check route table for 0.0.0.0/0 → nat-xxx
3. Security group allows outbound traffic? (Default allows all outbound)
4. Network ACL allows outbound traffic and inbound responses? (Check ephemeral ports)
5. Instance has public IP or Elastic IP? (Required for public subnet internet access)

## Adaptation Guidelines

**This skill provides systematic procedures, but adapt to user context**:
- If user already provided account/region, skip asking again
- If user mentioned specific symptom (e.g., "can't SSH"), prioritize network diagnostics
- If instance is stopped, skip performance analysis
- If user mentioned recent changes, focus investigation there
- Combine diagnostics if user indicates multiple issues

**LLM Decision-Making**:
- Use semantic understanding to match user's problem description to diagnostic procedures
- Don't blindly execute all steps - prioritize based on user's stated issue
- Adapt output format based on user's technical level
- Suggest additional diagnostics if initial results are inconclusive

## Additional Resources
For deep-dive investigations, load additional files:
- `invoke_skill('aws-ec2-troubleshooting', load_additional_files=['networking.md'])`: Detailed network troubleshooting
- `invoke_skill('aws-ec2-troubleshooting', load_additional_files=['storage.md'])`: EBS deep-dive procedures
- `invoke_skill('aws-ec2-troubleshooting', load_additional_files=['performance.md'])`: Performance tuning guide

## Success Criteria
- User's EC2 instance issue is identified with specific findings
- Clear remediation steps provided
- User can resolve issue or escalate to AWS support with diagnostic data
