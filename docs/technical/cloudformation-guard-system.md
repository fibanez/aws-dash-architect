# CloudFormation Guard System

## Overview

The CloudFormation Guard system provides real-time compliance validation for CloudFormation templates using the [AWS CloudFormation Guard](https://github.com/aws-cloudformation/cloudformation-guard) library. This system integrates directly with the cfn-guard library to validate templates against regulatory compliance rules.

## Architecture

### Core Components

**GuardValidator** (`src/app/cfn_guard.rs`):
- Main validation engine using cfn-guard library
- Memory management for large rule sets
- Validation result caching and processing

**GuardRepositoryManager** (`src/app/guard_repository_manager.rs`):
- Git-based repository access for Guard rules
- Replaces bulk downloading with direct repository cloning
- Manages rule file parsing and organization

**ComplianceProgramSelector** (`src/app/dashui/compliance_program_selector.rs`):
- UI component for selecting compliance programs
- Tag-based interface with search and filtering
- Category-based organization (Government, Healthcare, Financial, etc.)

**GuardViolationsWindow** (`src/app/dashui/guard_violations_window.rs`):
- Displays validation results with severity filtering
- Groups violations by resource or rule
- Shows violation details and exemption status

### Data Flow

```
1. User selects compliance programs → ComplianceProgramSelector
2. Programs mapped to Guard rule files → GuardRepositoryManager  
3. CloudFormation template + rules → GuardValidator (cfn-guard library)
4. Validation results processed → GuardViolation structs
5. Results displayed → GuardViolationsWindow
```

## Integration Points

### cfn-guard Library Usage

The system uses cfn-guard 3.1.2 with the following API:

```rust
use cfn_guard::{run_checks, ValidateInput};

// Create validation input
let template_input = ValidateInput {
    content: template_content,
    file_type: FileType::Yaml,
};

let rules_input = ValidateInput {
    content: rules_content, 
    file_type: FileType::Guard,
};

// Run validation
let results = run_checks(template_input, rules_input, false)?;
```

### Compliance Programs

Supported compliance frameworks:
- **NIST 800-53 R5** - Federal security controls
- **PCI-DSS** - Payment card industry standards
- **HIPAA** - Healthcare data protection  
- **SOC 2** - Service organization controls
- **FedRAMP** - Federal cloud security

## Memory Management

The system implements progressive validation and memory monitoring:
- **Chunked Rule Processing** - Process rule sets in memory-safe batches
- **Validation Caching** - Cache results to avoid re-validation
- **Memory Limits** - Monitor and limit memory usage during validation
- **Background Processing** - Run validations in separate threads

## Error Handling

**Rule Loading Errors**:
- Git repository access failures
- Rule file parsing errors
- Network connectivity issues

**Validation Errors**:
- Template parsing failures
- Guard rule execution errors
- Memory exhaustion during validation

## Testing Strategy

**Integration Tests**:
- Real cfn-guard library execution
- Actual AWS Guard rules from repository
- End-to-end validation workflow testing

**Unit Tests**:
- Rule parsing and organization
- Validation result processing
- UI component behavior

**Performance Tests**:
- Large template validation
- Multiple compliance program validation
- Memory usage monitoring

## Development Notes

**Migration from Bulk Downloader**:
The system previously used `BulkRuleDownloader` for downloading rules via GitHub API. This has been replaced with `GuardRepositoryManager` for direct Git repository access, providing more reliable rule access and better version control integration.

**Real cfn-guard Integration**:
Phase 0 replaced fake validation patterns with real cfn-guard library integration. The system now executes actual Guard rules against CloudFormation templates, providing genuine compliance validation results.

## Related Documentation

- [CloudFormation System](cloudformation-system.md) - Template parsing and management
- [UI Component Testing](ui-component-testing.md) - Testing the compliance UI components
- [Testing Patterns](testing-patterns.md) - Memory-safe testing approaches