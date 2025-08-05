//! AWS Bridge Tools Module
//!
//! This module contains individual tool implementations for the AWS bridge system.
//! Each tool is in its own file for better organization and maintainability.

pub mod aws_describe_log_groups;
pub mod aws_describe_resource;
pub mod aws_find_account;
pub mod aws_find_region;
pub mod aws_get_log_entries;
pub mod aws_get_log_events;
pub mod aws_list_resources;
pub mod create_task;
pub mod todo_read;
pub mod todo_write;

// Re-export all tools for easy access
pub use aws_describe_log_groups::AwsDescribeLogGroupsTool;
pub use aws_describe_resource::AwsDescribeResourceTool;
pub use aws_find_account::{set_global_aws_identity, AccountSearchResult, AwsFindAccountTool};
pub use aws_find_region::{AwsFindRegionTool, RegionSearchResult};
pub use aws_get_log_entries::AwsGetLogEntriesTool;
pub use aws_get_log_events::AwsGetLogEventsTool;
pub use aws_list_resources::AwsListResourcesTool;
pub use create_task::{ActiveTask, CreateTaskTool};
pub use todo_read::TodoReadTool;
pub use todo_write::{TodoItem, TodoPriority, TodoStatus, TodoWriteTool};
