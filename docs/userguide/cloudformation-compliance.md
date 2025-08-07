# CloudFormation Compliance Validation

AWS Dash Architect includes integrated CloudFormation Guard validation to help ensure your templates comply with security and governance standards.

## Selecting Compliance Programs

When creating a new project:

1. **Open Project Settings** - Use the command palette (Space) or project menu
2. **Navigate to Compliance** - Find the "Compliance Programs" section
3. **Add Programs** - Click "+ Add Programs" to open the selection interface
4. **Choose Standards** - Select from available compliance programs:
   - **NIST 800-53 R5** - Federal security controls
   - **PCI-DSS** - Payment card industry standards  
   - **HIPAA** - Healthcare data protection
   - **SOC 2** - Service organization controls
   - **FedRAMP** - Federal cloud security
5. **Apply Selection** - Selected programs appear as colored tags

## Understanding Violations Window

After validation runs, the violations window displays:

**Violation Summary:**
- Total violations found
- Breakdown by severity (Critical, High, Medium, Low)
- Pass/fail status for each compliance program

**Detailed View:**
- **Resource Name** - Which CloudFormation resource has the issue
- **Rule Name** - Specific compliance rule that failed
- **Violation Message** - Description of what needs to be fixed
- **Severity Level** - Color-coded priority (red=critical, yellow=medium, etc.)

**Filter Options:**
- Filter by severity level
- Show/hide exempted violations
- Group by resource or compliance program

## Quick Actions

- **Manual Validation** - Click "Validate Now" to check your template
- **Filter Results** - Use severity filters to focus on critical issues
- **Group Violations** - View by resource or compliance program
- **Review Rules** - See which specific compliance rules failed

## Tips

- Start with one compliance program to avoid overwhelming results
- Focus on Critical and High severity violations first
- Use CloudFormation Metadata to exempt specific resources when justified