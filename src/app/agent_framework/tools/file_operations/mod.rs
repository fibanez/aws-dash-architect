//! File Operations Tools - Tools for Page Builder agent file management
//!
//! This module provides file operation tools that allow Page Builder agents to:
//! - Read files from their workspace
//! - Write/create files in their workspace
//! - List files and directories
//! - Delete files
//! - Get API documentation
//!
//! All tools enforce workspace isolation and prevent directory traversal attacks.
//!
//! ## Workspace Types
//!
//! Tools support both disk-based and VFS-based workspaces:
//! - **Disk**: Persistent storage at `~/.local/share/awsdash/pages/{page_name}/`
//! - **VFS**: Temporary in-memory storage using pattern `vfs:{vfs_id}:{page_id}`

#![warn(clippy::all, rust_2018_idioms)]

mod copy_file;
mod delete_file;
mod edit_file;
mod get_api_docs;
mod list_files;
mod open_page;
mod read_file;
mod workspace;
mod write_file;

pub use copy_file::CopyFileTool;
pub use delete_file::DeleteFileTool;
pub use edit_file::EditFileTool;
pub use get_api_docs::GetApiDocsTool;
pub use list_files::ListFilesTool;
pub use open_page::OpenPageTool;
pub use read_file::ReadFileTool;
pub use workspace::{WorkspaceFileEntry, WorkspaceType};
pub use write_file::WriteFileTool;
