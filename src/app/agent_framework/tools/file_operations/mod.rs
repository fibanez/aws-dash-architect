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

#![warn(clippy::all, rust_2018_idioms)]

mod delete_file;
mod edit_file;
mod get_api_docs;
mod list_files;
mod open_page;
mod read_file;
mod write_file;

pub use delete_file::DeleteFileTool;
pub use edit_file::EditFileTool;
pub use get_api_docs::GetApiDocsTool;
pub use list_files::ListFilesTool;
pub use open_page::OpenPageTool;
pub use read_file::ReadFileTool;
pub use write_file::WriteFileTool;
