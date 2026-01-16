//! VFS function bindings for JavaScript execution
//!
//! Provides JavaScript access to the Virtual File System for reading/writing
//! files, listing directories, and managing agent memory.

#![warn(clippy::all, rust_2018_idioms)]

use crate::app::agent_framework::vfs::{get_current_vfs_id, with_vfs, with_vfs_mut};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Directory entry information returned to JavaScript
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsDirEntry {
    pub name: String,
    #[serde(rename = "type")]
    pub entry_type: String, // "file" or "directory"
    pub size: usize,
}

/// File stat information returned to JavaScript
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsFileStat {
    pub size: usize,
    pub is_directory: bool,
    pub is_file: bool,
}

/// Register VFS functions into V8 context
pub fn register(scope: &mut v8::ContextScope<'_, '_, v8::HandleScope<'_>>) -> Result<()> {
    let global = scope.get_current_context().global(scope);

    // Create vfs namespace object
    let vfs_obj = v8::Object::new(scope);

    // Register vfs.readFile(path) -> string
    let read_file_fn = v8::Function::new(scope, read_file_callback)
        .expect("Failed to create vfs.readFile function");
    let fn_name = v8::String::new(scope, "readFile").expect("Failed to create function name");
    vfs_obj.set(scope, fn_name.into(), read_file_fn.into());

    // Register vfs.writeFile(path, content)
    let write_file_fn = v8::Function::new(scope, write_file_callback)
        .expect("Failed to create vfs.writeFile function");
    let fn_name = v8::String::new(scope, "writeFile").expect("Failed to create function name");
    vfs_obj.set(scope, fn_name.into(), write_file_fn.into());

    // Register vfs.listDir(path) -> array
    let list_dir_fn =
        v8::Function::new(scope, list_dir_callback).expect("Failed to create vfs.listDir function");
    let fn_name = v8::String::new(scope, "listDir").expect("Failed to create function name");
    vfs_obj.set(scope, fn_name.into(), list_dir_fn.into());

    // Register vfs.mkdir(path)
    let mkdir_fn =
        v8::Function::new(scope, mkdir_callback).expect("Failed to create vfs.mkdir function");
    let fn_name = v8::String::new(scope, "mkdir").expect("Failed to create function name");
    vfs_obj.set(scope, fn_name.into(), mkdir_fn.into());

    // Register vfs.exists(path) -> boolean
    let exists_fn =
        v8::Function::new(scope, exists_callback).expect("Failed to create vfs.exists function");
    let fn_name = v8::String::new(scope, "exists").expect("Failed to create function name");
    vfs_obj.set(scope, fn_name.into(), exists_fn.into());

    // Register vfs.stat(path) -> object
    let stat_fn =
        v8::Function::new(scope, stat_callback).expect("Failed to create vfs.stat function");
    let fn_name = v8::String::new(scope, "stat").expect("Failed to create function name");
    vfs_obj.set(scope, fn_name.into(), stat_fn.into());

    // Register vfs.delete(path)
    let delete_fn =
        v8::Function::new(scope, delete_callback).expect("Failed to create vfs.delete function");
    let fn_name = v8::String::new(scope, "delete").expect("Failed to create function name");
    vfs_obj.set(scope, fn_name.into(), delete_fn.into());

    // Add vfs object to global scope
    let vfs_name = v8::String::new(scope, "vfs").expect("Failed to create vfs name");
    global.set(scope, vfs_name.into(), vfs_obj.into());

    Ok(())
}

/// Get documentation for VFS functions
pub fn get_documentation() -> String {
    r#"### vfs - Virtual File System

The `vfs` object provides access to the agent's virtual file system for storing and retrieving data.

#### vfs.readFile(path: string, options?: {offset?: number, length?: number}): string
Read a file from VFS and return its content as a string.
For files >100KB, you must use offset/length to read in chunks.
```javascript
// Small files (< 100KB)
const content = vfs.readFile("/results/small.json");
const data = JSON.parse(content);

// Large files - use chunked reading
const stat = vfs.stat("/results/large.json");
const chunk = vfs.readFile("/results/large.json", { offset: 0, length: 50000 });
```

#### vfs.writeFile(path: string, content: string): void
Write string content to a file in VFS. Creates parent directories automatically.
```javascript
// Write plain text
vfs.writeFile("/workspace/temp.txt", "Hello, World!");

// Write JSON (stringify first)
vfs.writeFile("/workspace/result.json", JSON.stringify({ count: 42, items: [] }));
```

#### vfs.listDir(path: string): Array<{name: string, type: string, size: number}>
List entries in a directory.
```javascript
const entries = vfs.listDir("/results");
entries.forEach(e => console.log(`${e.name} (${e.type})`));
```

#### vfs.mkdir(path: string): void
Create a directory.
```javascript
vfs.mkdir("/workspace/processed");
```

#### vfs.exists(path: string): boolean
Check if a path exists.
```javascript
if (vfs.exists("/results/cache.json")) { ... }
```

#### vfs.stat(path: string): {size: number, isDirectory: boolean, isFile: boolean}
Get file/directory information.
```javascript
const info = vfs.stat("/results/data.json");
console.log(`Size: ${info.size} bytes`);
```

#### vfs.delete(path: string): void
Delete a file or empty directory.
```javascript
vfs.delete("/workspace/temp.txt");
```
"#
    .to_string()
}

// Helper to throw VFS error
fn throw_vfs_error(scope: &mut v8::PinScope<'_, '_>, msg: &str) {
    let v8_msg = v8::String::new(scope, msg).unwrap();
    let error = v8::Exception::error(scope, v8_msg);
    scope.throw_exception(error);
}

// Helper to get VFS ID or throw
fn get_vfs_id_or_throw(scope: &mut v8::PinScope<'_, '_>) -> Option<String> {
    match get_current_vfs_id() {
        Some(id) => Some(id),
        None => {
            throw_vfs_error(scope, "VFS not available - no VFS ID set for this context");
            None
        }
    }
}

// Helper to get string argument
fn get_string_arg(
    scope: &mut v8::PinScope<'_, '_>,
    args: &v8::FunctionCallbackArguments<'_>,
    index: i32,
    name: &str,
) -> Option<String> {
    if args.length() <= index {
        throw_vfs_error(scope, &format!("Missing argument: {}", name));
        return None;
    }

    let arg = args.get(index);
    if let Some(s) = arg.to_string(scope) {
        Some(s.to_rust_string_lossy(scope))
    } else {
        throw_vfs_error(scope, &format!("Invalid argument {}: expected string", name));
        None
    }
}

/// Maximum file size (100KB) that can be read without chunking
const MAX_UNCHUNKED_READ_SIZE: usize = 100 * 1024;

/// Callback for vfs.readFile(path, options?)
/// Options: { offset?: number, length?: number }
/// If file >100KB and no offset/length, returns error instructing chunked reading
fn read_file_callback(
    scope: &mut v8::PinScope<'_, '_>,
    args: v8::FunctionCallbackArguments<'_>,
    mut rv: v8::ReturnValue<'_>,
) {
    let vfs_id = match get_vfs_id_or_throw(scope) {
        Some(id) => id,
        None => return,
    };

    let path = match get_string_arg(scope, &args, 0, "path") {
        Some(p) => p,
        None => return,
    };

    // Parse optional options argument: { offset?: number, length?: number }
    let (offset, length) = if args.length() > 1 && args.get(1).is_object() {
        let opts = args.get(1).to_object(scope).unwrap();

        let offset_key = v8::String::new(scope, "offset").unwrap();
        let length_key = v8::String::new(scope, "length").unwrap();

        let offset = opts.get(scope, offset_key.into())
            .and_then(|v| if v.is_number() { v.number_value(scope).map(|n| n as usize) } else { None });
        let length = opts.get(scope, length_key.into())
            .and_then(|v| if v.is_number() { v.number_value(scope).map(|n| n as usize) } else { None });

        (offset, length)
    } else {
        (None, None)
    };

    // First, check file size if no chunking params provided
    if offset.is_none() && length.is_none() {
        let size_result = with_vfs(&vfs_id, |vfs| vfs.stat(&path).map(|m| m.size));

        if let Some(Ok(size)) = size_result {
            if size > MAX_UNCHUNKED_READ_SIZE {
                let error_msg = format!(
                    "File too large to read at once ({} bytes, max {} bytes).\n\
                    Use chunked reading with offset and length parameters:\n\n\
                    // Check file size first\n\
                    const stat = vfs.stat('{}');\n\
                    console.log('File size:', stat.size);\n\n\
                    // Read in chunks (e.g., 50KB at a time)\n\
                    const chunk1 = vfs.readFile('{}', {{ offset: 0, length: 50000 }});\n\
                    const chunk2 = vfs.readFile('{}', {{ offset: 50000, length: 50000 }});\n\
                    // ... continue until all data read",
                    size, MAX_UNCHUNKED_READ_SIZE, path, path, path
                );
                throw_vfs_error(scope, &error_msg);
                return;
            }
        }
    }

    // Read the file (with optional chunking)
    let result = with_vfs(&vfs_id, |vfs| -> anyhow::Result<Vec<u8>> {
        let bytes = vfs.read_file(&path)?;

        // Apply offset and length if provided
        let start = offset.unwrap_or(0);
        let end = match length {
            Some(len) => std::cmp::min(start + len, bytes.len()),
            None => bytes.len(),
        };

        if start >= bytes.len() {
            return Ok(Vec::new()); // Offset beyond file size returns empty
        }

        Ok(bytes[start..end].to_vec())
    });

    match result {
        Some(Ok(bytes)) => {
            let content = String::from_utf8_lossy(&bytes);
            if let Some(v8_str) = v8::String::new(scope, &content) {
                rv.set(v8_str.into());
            } else {
                throw_vfs_error(scope, "Failed to create V8 string");
            }
        }
        Some(Err(e)) => throw_vfs_error(scope, &e.to_string()),
        None => throw_vfs_error(scope, &format!("VFS not found: {}", vfs_id)),
    }
}

/// Callback for vfs.writeFile(path, content)
fn write_file_callback(
    scope: &mut v8::PinScope<'_, '_>,
    args: v8::FunctionCallbackArguments<'_>,
    _rv: v8::ReturnValue<'_>,
) {
    let vfs_id = match get_vfs_id_or_throw(scope) {
        Some(id) => id,
        None => return,
    };

    let path = match get_string_arg(scope, &args, 0, "path") {
        Some(p) => p,
        None => return,
    };

    let content = match get_string_arg(scope, &args, 1, "content") {
        Some(c) => c,
        None => return,
    };

    let result = with_vfs_mut(&vfs_id, |vfs| vfs.write_file(&path, content.as_bytes()));

    match result {
        Some(Ok(())) => {}
        Some(Err(e)) => throw_vfs_error(scope, &e.to_string()),
        None => throw_vfs_error(scope, &format!("VFS not found: {}", vfs_id)),
    }
}

/// Callback for vfs.listDir(path)
fn list_dir_callback(
    scope: &mut v8::PinScope<'_, '_>,
    args: v8::FunctionCallbackArguments<'_>,
    mut rv: v8::ReturnValue<'_>,
) {
    let vfs_id = match get_vfs_id_or_throw(scope) {
        Some(id) => id,
        None => return,
    };

    let path = match get_string_arg(scope, &args, 0, "path") {
        Some(p) => p,
        None => return,
    };

    let result = with_vfs(&vfs_id, |vfs| {
        vfs.list_dir(&path).map(|entries| {
            entries
                .into_iter()
                .map(|e| JsDirEntry {
                    name: e.name,
                    entry_type: if e.is_directory {
                        "directory".to_string()
                    } else {
                        "file".to_string()
                    },
                    size: e.size,
                })
                .collect::<Vec<_>>()
        })
    });

    match result {
        Some(Ok(entries)) => {
            let json_str = match serde_json::to_string(&entries) {
                Ok(s) => s,
                Err(e) => {
                    throw_vfs_error(scope, &format!("Failed to serialize entries: {}", e));
                    return;
                }
            };

            if let Some(v8_str) = v8::String::new(scope, &json_str) {
                match v8::json::parse(scope, v8_str) {
                    Some(parsed) => rv.set(parsed),
                    None => throw_vfs_error(scope, "Failed to parse JSON"),
                }
            } else {
                throw_vfs_error(scope, "Failed to create V8 string");
            }
        }
        Some(Err(e)) => throw_vfs_error(scope, &e.to_string()),
        None => throw_vfs_error(scope, &format!("VFS not found: {}", vfs_id)),
    }
}

/// Callback for vfs.mkdir(path)
fn mkdir_callback(
    scope: &mut v8::PinScope<'_, '_>,
    args: v8::FunctionCallbackArguments<'_>,
    _rv: v8::ReturnValue<'_>,
) {
    let vfs_id = match get_vfs_id_or_throw(scope) {
        Some(id) => id,
        None => return,
    };

    let path = match get_string_arg(scope, &args, 0, "path") {
        Some(p) => p,
        None => return,
    };

    let result = with_vfs_mut(&vfs_id, |vfs| vfs.mkdir(&path));

    match result {
        Some(Ok(())) => {}
        Some(Err(e)) => throw_vfs_error(scope, &e.to_string()),
        None => throw_vfs_error(scope, &format!("VFS not found: {}", vfs_id)),
    }
}

/// Callback for vfs.exists(path)
fn exists_callback(
    scope: &mut v8::PinScope<'_, '_>,
    args: v8::FunctionCallbackArguments<'_>,
    mut rv: v8::ReturnValue<'_>,
) {
    let vfs_id = match get_vfs_id_or_throw(scope) {
        Some(id) => id,
        None => return,
    };

    let path = match get_string_arg(scope, &args, 0, "path") {
        Some(p) => p,
        None => return,
    };

    let result = with_vfs(&vfs_id, |vfs| vfs.exists(&path));

    match result {
        Some(exists) => {
            let v8_bool = v8::Boolean::new(scope, exists);
            rv.set(v8_bool.into());
        }
        None => throw_vfs_error(scope, &format!("VFS not found: {}", vfs_id)),
    }
}

/// Callback for vfs.stat(path)
fn stat_callback(
    scope: &mut v8::PinScope<'_, '_>,
    args: v8::FunctionCallbackArguments<'_>,
    mut rv: v8::ReturnValue<'_>,
) {
    let vfs_id = match get_vfs_id_or_throw(scope) {
        Some(id) => id,
        None => return,
    };

    let path = match get_string_arg(scope, &args, 0, "path") {
        Some(p) => p,
        None => return,
    };

    let result = with_vfs(&vfs_id, |vfs| {
        vfs.stat(&path).map(|meta| JsFileStat {
            size: meta.size,
            is_directory: meta.is_directory,
            is_file: !meta.is_directory,
        })
    });

    match result {
        Some(Ok(stat)) => {
            let json_str = match serde_json::to_string(&stat) {
                Ok(s) => s,
                Err(e) => {
                    throw_vfs_error(scope, &format!("Failed to serialize stat: {}", e));
                    return;
                }
            };

            if let Some(v8_str) = v8::String::new(scope, &json_str) {
                match v8::json::parse(scope, v8_str) {
                    Some(parsed) => rv.set(parsed),
                    None => throw_vfs_error(scope, "Failed to parse JSON"),
                }
            } else {
                throw_vfs_error(scope, "Failed to create V8 string");
            }
        }
        Some(Err(e)) => throw_vfs_error(scope, &e.to_string()),
        None => throw_vfs_error(scope, &format!("VFS not found: {}", vfs_id)),
    }
}

/// Callback for vfs.delete(path)
fn delete_callback(
    scope: &mut v8::PinScope<'_, '_>,
    args: v8::FunctionCallbackArguments<'_>,
    _rv: v8::ReturnValue<'_>,
) {
    let vfs_id = match get_vfs_id_or_throw(scope) {
        Some(id) => id,
        None => return,
    };

    let path = match get_string_arg(scope, &args, 0, "path") {
        Some(p) => p,
        None => return,
    };

    let result = with_vfs_mut(&vfs_id, |vfs| vfs.delete(&path));

    match result {
        Some(Ok(())) => {}
        Some(Err(e)) => throw_vfs_error(scope, &e.to_string()),
        None => throw_vfs_error(scope, &format!("VFS not found: {}", vfs_id)),
    }
}
