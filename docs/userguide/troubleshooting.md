# Troubleshooting

Comprehensive guide for diagnosing and resolving common issues in AWS Dash Architect.

## General Troubleshooting Approach

### Diagnostic Information

*Application Logs*:
* Location: `$HOME/.local/share/awsdash/logs/awsdash.log`
* Contains both `log` and `tracing` output
* Includes AWS SDK debug information for Bedrock
* Automatically rotated to prevent disk space issues

*Debug Mode*:
* Enhanced AWS SDK logging enabled by default
* Detailed operation tracing for complex workflows
* Performance metrics for optimization
* Error context preservation for debugging

### Common Diagnostic Steps

1. *Check Application Logs* - Review recent log entries for errors
2. *Verify System Requirements* - Ensure compatible OS and dependencies
3. *Test Network Connectivity* - Verify internet access for AWS operations
4. *Check File Permissions* - Ensure read/write access to project directories
5. *Validate AWS Configuration* - Confirm credentials and region settings

## AWS Authentication Issues

### Identity Center Login Problems

*Symptoms*: Cannot authenticate with AWS Identity Center
* Device authorization flow fails to complete
* Browser doesn't open for authentication
* Authentication completes but credentials not received

*Diagnostic Steps*:
1. Check network connectivity to AWS services
2. Review application logs for authentication errors
3. Verify Identity Center configuration in AWS console
4. Test browser access to AWS Identity Center

*Solutions*:
* **Network Issues**: Configure proxy settings or firewall rules
* **Browser Problems**: Try different browser or private/incognito mode  
* **Identity Center Config**: Verify application registration in AWS console
* **Credential Expiry**: Re-authenticate through the login window

### Multi-Account Access Problems

*Symptoms*: Cannot access resources in secondary accounts
* Account list doesn't populate
* Role assumption fails
* Credentials work for one account but not others

*Diagnostic Steps*:
1. Verify account permissions in AWS console
2. Check role trust relationships for cross-account access
3. Review credential debug window for token details
4. Test account switching in AWS console directly

*Solutions*:
* **Permission Issues**: Update IAM policies for cross-account access
* **Role Trust**: Configure proper trust relationships in target accounts
* **Credential Refresh**: Re-authenticate to refresh account list
* **Regional Settings**: Ensure consistent region configuration

### Credential Expiration Issues

*Symptoms*: Operations fail with authentication errors
* API calls return 401/403 errors
* Credential debug shows expired tokens
* Re-authentication required frequently

*Diagnostic Steps*:
1. Check credential expiration times in debug window
2. Review automatic refresh behavior in logs
3. Verify system clock accuracy
4. Test credential refresh mechanism

*Solutions*:
* **Clock Sync**: Ensure system clock is accurate
* **Refresh Settings**: Adjust credential refresh buffer times
* **Manual Refresh**: Re-authenticate through login window
* **Session Length**: Request longer session duration in Identity Center

## CloudFormation Template Issues

### Template Parsing Errors

*Symptoms*: Templates fail to load or display errors
* JSON/YAML syntax errors
* Unrecognized resource types or properties
* Template structure validation failures

*Diagnostic Steps*:
1. Validate template syntax with external tools
2. Check resource type availability in selected region
3. Review template against CloudFormation documentation
4. Test template in AWS console directly

*Solutions*:
* **Syntax Errors**: Use JSON/YAML validators to fix structure
* **Resource Availability**: Verify resource types support in target region
* **Schema Updates**: Download latest AWS resource specifications
* **Template Version**: Ensure CloudFormation template format version compatibility

### Dependency Validation Problems

*Symptoms*: Dependency graph shows errors or fails to build
* Circular dependency warnings
* Missing dependency relationships
* Graph rendering issues

*Diagnostic Steps*:
1. Review dependency graph in visualization window
2. Check for circular references in template
3. Validate resource reference syntax
4. Test dependency order with deployment simulation

*Solutions*:
* **Circular Dependencies**: Restructure template to eliminate cycles
* **Reference Syntax**: Fix Ref and GetAtt function usage
* **Missing Dependencies**: Add explicit DependsOn declarations
* **Graph Corruption**: Use emergency recovery in verification window

### Resource Configuration Issues

*Symptoms*: Resource forms show validation errors
* Property validation failures
* Required fields not enforced
* Schema constraint violations

*Diagnostic Steps*:
1. Review resource documentation for property requirements
2. Check constraint validation in property forms
3. Verify resource type schema compatibility
4. Test configuration in AWS console

*Solutions*:
* **Required Properties**: Ensure all required fields are completed
* **Data Types**: Verify property values match expected types
* **Constraints**: Review and fix constraint violations
* **Schema Updates**: Download latest resource specifications

## Project Management Issues

### File Operations Problems

*Symptoms*: Cannot save, load, or export projects
* File permission errors
* Corrupt project files
* Import/export failures

*Diagnostic Steps*:
1. Check file system permissions for project directory
2. Verify disk space availability
3. Test file operations with simple projects
4. Review file format compatibility

*Solutions*:
* **Permissions**: Ensure read/write access to project directories
* **Disk Space**: Free up disk space or choose different location
* **File Corruption**: Use backup copies or emergency recovery
* **Format Issues**: Convert projects to supported formats

### Resource Import Problems

*Symptoms*: Resources don't import correctly from templates
* Property type conversion errors
* Dependency resolution failures
* Metadata loss during import

*Diagnostic Steps*:
1. Review import process in application logs
2. Check template compatibility with project format
3. Verify resource type support
4. Test with simpler template structures

*Solutions*:
* **Type Conversion**: Update property type handling for complex values
* **Dependencies**: Import resources in dependency order
* **Metadata Preservation**: Use template sections for additional metadata
* **Batch Processing**: Import large templates in smaller batches

### Environment Management Issues

*Symptoms*: Environment configuration problems
* Environment switching failures
* Resource deployment errors
* Cross-environment conflicts

*Diagnostic Steps*:
1. Verify environment configuration settings
2. Check AWS credentials for each environment
3. Review resource naming and tag consistency
4. Test environment isolation

*Solutions*:
* **Configuration**: Update environment settings for correct regions/accounts
* **Credentials**: Ensure proper AWS access for each environment
* **Naming**: Use consistent naming conventions across environments
* **Isolation**: Implement proper resource tagging and separation

## User Interface Issues

### Window Management Problems

*Symptoms*: Windows don't behave correctly
* Windows not focusing when selected
* Multiple windows of same type
* Window state not persisting

*Diagnostic Steps*:
1. Check window focus manager behavior
2. Review window selector functionality
3. Test window state persistence
4. Verify trait implementation for custom windows

*Solutions*:
* **Focus Issues**: Verify focus manager configuration
* **Duplicate Windows**: Ensure singleton pattern for unique windows
* **State Persistence**: Check serialization of window preferences
* **Custom Windows**: Review trait implementation compliance

### Performance Problems

*Symptoms*: Slow UI response or high resource usage
* Slow rendering or interaction lag
* High memory usage
* CPU spikes during operations

*Diagnostic Steps*:
1. Profile application performance with system tools
2. Review memory usage patterns
3. Identify expensive operations in logs
4. Test with smaller datasets

*Solutions*:
* **Rendering**: Optimize UI updates and reduce redraws
* **Memory**: Review caching strategy and cleanup procedures
* **Processing**: Move expensive operations to background threads
* **Data Size**: Implement pagination or lazy loading for large datasets

### Theme and Visual Issues

*Symptoms*: Visual appearance problems
* Inconsistent styling across windows
* Missing icons or graphics
* Theme switching problems

*Diagnostic Steps*:
1. Verify theme application across components
2. Check asset loading and caching
3. Test theme switching behavior
4. Review custom styling implementation

*Solutions*:
* **Styling**: Ensure consistent theme application
* **Assets**: Verify icon and graphic file availability
* **Theme Switching**: Fix theme persistence and application
* **Custom Styling**: Update custom components for theme compatibility

## Resource Specification Issues

### Download Problems

*Symptoms*: AWS resource specifications don't download
* Network timeouts during download
* Authentication failures for resource access
* Incomplete or corrupt specification files

*Diagnostic Steps*:
1. Test network connectivity to AWS documentation
2. Review download manager status and logs
3. Check available disk space
4. Verify file write permissions

*Solutions*:
* **Network**: Configure proxy or firewall settings
* **Authentication**: Update AWS credentials for resource access
* **Storage**: Ensure adequate disk space for specifications
* **Permissions**: Fix file system write permissions

### Schema Validation Problems

*Symptoms*: Resource forms don't generate correctly
* Missing property fields
* Incorrect validation rules
* Schema parsing errors

*Diagnostic Steps*:
1. Review resource specification file integrity
2. Check schema parsing logic for errors
3. Verify resource type classification
4. Test with known good specifications

*Solutions*:
* **File Integrity**: Re-download corrupted specifications
* **Parsing Logic**: Update schema parser for new formats
* **Classification**: Fix resource type vs property type detection
* **Validation**: Update constraint parsing for new schema features

## Error Recovery Procedures

### Emergency Data Recovery

*For Corrupt Project Files*:
1. Check for automatic backup files in project directory
2. Use emergency recovery methods in project management
3. Restore from version control if available
4. Reconstruct from CloudFormation templates

*For Corrupt Application State*:
1. Clear application cache and temporary files
2. Reset window layouts and preferences
3. Re-download AWS resource specifications
4. Restart with clean configuration

### Factory Reset Procedures

*Complete Reset* (preserves projects):
1. Close application completely
2. Delete configuration directory: `$HOME/.local/share/awsdash/`
3. Keep project files in separate directories
4. Restart application for fresh configuration

*Partial Reset* (preserves preferences):
1. Clear specific cache directories
2. Reset only problematic configurations
3. Maintain user preferences and themes
4. Selective component reinitialization

## Getting Additional Help

### Log Analysis

*Important Log Sections*:
* Authentication flows and credential management
* Template parsing and validation operations
* AWS SDK interactions and error responses
* UI event processing and window management

*Log Sharing*:
* Remove sensitive information before sharing
* Include relevant timestamp ranges
* Provide context about operations being performed
* Include system information and application version

### Community Support

* GitHub Issues: Report bugs and feature requests
* Documentation: Check latest documentation updates
* Community Forums: Ask questions and share solutions
* Stack Overflow: Technical implementation questions

### Professional Support

For enterprise users requiring professional support:
* Include detailed error logs and reproduction steps
* Provide environment and configuration details
* Describe business impact and urgency level
* Include contact information for follow-up

## Related Documentation

* [System Architecture](system-architecture.md)
* [User Interface Guide](user-interface.md)
* [Performance Optimization](performance-optimization.md)
* [Development Guide](development-guide.md)