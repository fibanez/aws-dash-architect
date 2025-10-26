# Troubleshooting

Comprehensive guide for diagnosing and resolving common issues in AWS Dash.

## General Troubleshooting Approach

### Diagnostic Information

*Application Logs*:
* Location: `$HOME/.local/share/awsdash/logs/awsdash.log`
* Contains both `log` and `tracing` output
* Includes AWS SDK debug information
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

## AWS Resource Explorer Issues

### Resource Discovery Problems

*Symptoms*: Resources not appearing or incomplete listings
* Missing resources in specific regions
* Account switching issues
* Resource cache outdated

*Diagnostic Steps*:
1. Verify account permissions for resource access
2. Check selected regions in Explorer configuration
3. Review application logs for API errors
4. Test resource access in AWS console directly

*Solutions*:
* **Permissions**: Ensure IAM policies allow resource discovery API calls
* **Region Selection**: Verify correct regions are enabled for scanning
* **Cache Refresh**: Clear resource cache and reload
* **API Throttling**: Review rate limiting and implement retry logic

### Resource Details and Metadata

*Symptoms*: Resource details incomplete or not displaying
* Property values missing or incorrect
* Metadata not loading
* Resource relationships not shown

*Diagnostic Steps*:
1. Check AWS API response in application logs
2. Verify resource type is supported
3. Test describe operations in AWS CLI
4. Review resource normalization logic

*Solutions*:
* **API Support**: Verify resource type has describe API
* **Property Mapping**: Check resource normalizer implementation
* **Metadata Loading**: Review caching and refresh logic
* **Relationships**: Verify dependency analysis configuration

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

## Control Bridge Issues

### AI Agent Communication Problems

*Symptoms*: Bridge agent not responding or errors
* Agent task execution failures
* Tool invocation errors
* Response parsing issues

*Diagnostic Steps*:
1. Check Bridge window logs for errors
2. Verify AI service connectivity
3. Review task execution status
4. Test with simpler queries

*Solutions*:
* **Network**: Verify connectivity to AI service endpoints
* **Authentication**: Check AI service credentials configuration
* **Tool Errors**: Review tool implementation and permissions
* **Task Complexity**: Break down complex operations into smaller steps

## Error Recovery Procedures

### Emergency Data Recovery

*For Corrupt Application State*:
1. Clear application cache and temporary files
2. Reset window layouts and preferences
3. Restart with clean configuration
4. Check application logs for corruption sources

### Factory Reset Procedures

*Complete Reset*:
1. Close application completely
2. Delete configuration directory: `$HOME/.local/share/awsdash/`
3. Restart application for fresh configuration

*Partial Reset* (preserves preferences):
1. Clear specific cache directories
2. Reset only problematic configurations
3. Maintain user preferences and themes
4. Selective component reinitialization

## Getting Additional Help

### Log Analysis

*Important Log Sections*:
* Authentication flows and credential management
* AWS resource discovery and API operations
* Control Bridge agent task execution
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

* [Setup IAM Identity Center](setup-iam-identity-center.md) - AWS authentication configuration
* [Technical Documentation](../technical/README.md) - System architecture and development guides