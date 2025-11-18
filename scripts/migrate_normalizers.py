#!/usr/bin/env python3
"""
Migrate normalizers to AsyncResourceNormalizer trait.

This script automates the migration of normalizers from sync ResourceNormalizer
to async AsyncResourceNormalizer with tag fetching support.
"""

import re
import sys
from pathlib import Path


def migrate_normalizer_file(file_path: Path) -> bool:
    """Migrate a single normalizer file."""
    print(f"Processing: {file_path}")

    content = file_path.read_text()
    original_content = content

    # Step 1: Add async_trait import if not present
    if 'use async_trait::async_trait;' not in content:
        # Find the imports section and add async_trait
        if 'use anyhow::Result;' in content:
            content = content.replace(
                'use anyhow::Result;',
                'use anyhow::Result;\nuse async_trait::async_trait;'
            )
        elif 'use chrono::{DateTime, Utc};' in content:
            content = content.replace(
                'use chrono::{DateTime, Utc};',
                'use async_trait::async_trait;\nuse chrono::{DateTime, Utc};'
            )

    # Step 2: Find all ResourceNormalizer implementations
    # Pattern: impl ResourceNormalizer for XxxNormalizer {
    pattern = r'impl ResourceNormalizer for (\w+Normalizer) \{'
    matches = list(re.finditer(pattern, content))

    if not matches:
        print(f"  No ResourceNormalizer implementations found")
        return False

    print(f"  Found {len(matches)} normalizer(s) to migrate")

    # Process each normalizer from bottom to top (to preserve positions)
    for match in reversed(matches):
        normalizer_name = match.group(1)
        start_pos = match.start()

        # Find the normalize() method
        # Look for: fn normalize(\n        &self,\n        raw_response: serde_json::Value,
        normalize_pattern = (
            r'fn normalize\(\s*'
            r'&self,\s*'
            r'raw_response: serde_json::Value,\s*'
            r'account: &str,\s*'
            r'region: &str,\s*'
            r'query_timestamp: DateTime<Utc>,\s*'
            r'\) -> Result<ResourceEntry> \{'
        )

        # Search for normalize method after the impl start
        impl_section = content[start_pos:start_pos + 5000]  # Look ahead up to 5000 chars
        normalize_match = re.search(normalize_pattern, impl_section, re.MULTILINE)

        if not normalize_match:
            print(f"  WARNING: Could not find normalize() method for {normalizer_name}")
            continue

        # Find the line with `let tags = extract_tags(&raw_response);`
        tags_pattern = r'(\s+)let tags = extract_tags\(&raw_response\);'
        tags_match = re.search(tags_pattern, impl_section)

        if not tags_match:
            print(f"  WARNING: Could not find tags extraction for {normalizer_name}")
            continue

        indent = tags_match.group(1)
        tags_line_start = start_pos + tags_match.start()
        tags_line_end = start_pos + tags_match.end()

        # Extract resource_id variable name (look backwards from tags line)
        resource_id_pattern = r'let (\w+) = raw_response'
        id_matches = list(re.finditer(resource_id_pattern, content[start_pos:tags_line_start]))
        if not id_matches:
            print(f"  WARNING: Could not find resource_id variable for {normalizer_name}")
            continue

        resource_id_var = id_matches[0].group(1)  # Use first match (usually the main ID)

        # Get resource type from resource_type() method
        resource_type_pattern = r'fn resource_type\(&self\) -> &\'static str \{\s*"([^"]+)"'
        type_match = re.search(resource_type_pattern, impl_section)
        if not type_match:
            print(f"  WARNING: Could not find resource_type for {normalizer_name}")
            continue

        resource_type = type_match.group(1)

        # Generate async tag fetching code
        async_tags_code = f'''{indent}// Fetch tags asynchronously from AWS API with caching
{indent}let tags = aws_client
{indent}    .fetch_tags_for_resource("{resource_type}", &{resource_id_var}, account, region)
{indent}    .await
{indent}    .unwrap_or_else(|e| {{
{indent}        tracing::warn!("Failed to fetch tags for {resource_type} {{}}: {{}}", {resource_id_var}, e);
{indent}        Vec::new()
{indent}    }});'''

        # Replace the extract_tags line
        content = content[:tags_line_start] + async_tags_code + content[tags_line_end:]

        # Now add async trait implementation BEFORE the sync impl
        async_impl_code = f'''#[async_trait]
impl AsyncResourceNormalizer for {normalizer_name} {{
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {{'''

        # Find where to insert (right before "impl ResourceNormalizer")
        impl_insertion_point = content.find(f'impl ResourceNormalizer for {normalizer_name}')
        if impl_insertion_point == -1:
            print(f"  ERROR: Lost track of impl for {normalizer_name}")
            continue

        # Adjust for changes made so far
        updated_impl_section = content[impl_insertion_point:impl_insertion_point + 5000]

        # Extract the entire implementation (find matching closing brace)
        brace_count = 0
        impl_end = impl_insertion_point
        found_start = False

        for i, char in enumerate(content[impl_insertion_point:]):
            if char == '{':
                brace_count += 1
                found_start = True
            elif char == '}':
                brace_count -= 1
                if found_start and brace_count == 0:
                    impl_end = impl_insertion_point + i + 1
                    break

        if impl_end == impl_insertion_point:
            print(f"  ERROR: Could not find end of impl for {normalizer_name}")
            continue

        # Get the original implementation
        original_impl = content[impl_insertion_point:impl_end]

        # Create async version by modifying the original
        async_impl = original_impl

        # Change impl declaration to async
        async_impl = async_impl.replace(
            f'impl ResourceNormalizer for {normalizer_name}',
            f'#[async_trait]\nimpl AsyncResourceNormalizer for {normalizer_name}',
            1
        )

        # Change fn signature to async
        async_impl = re.sub(
            r'fn normalize\(\s*&self,\s*raw_response: serde_json::Value,\s*account: &str,\s*region: &str,\s*query_timestamp: DateTime<Utc>,\s*\)',
            'async fn normalize(\n        &self,\n        raw_response: serde_json::Value,\n        account: &str,\n        region: &str,\n        query_timestamp: DateTime<Utc>,\n        aws_client: &AWSResourceClient,\n    )',
            async_impl
        )

        # Add comment to original impl
        sync_impl_comment = f'''// Temporary: Keep old sync implementation for compatibility during migration
// This will be removed once all normalizers are migrated and query_resources is updated
#[allow(deprecated)]
'''

        # Replace old impl with async impl + commented sync impl
        modified_impl = f'''{async_impl}

{sync_impl_comment}{original_impl.replace(async_tags_code, f'{indent}let tags = extract_tags(&raw_response); // Fallback to local extraction for sync path')}'''

        content = content[:impl_insertion_point] + modified_impl + content[impl_end:]

    # Only write if changes were made
    if content != original_content:
        file_path.write_text(content)
        print(f"  ✓ Migrated {len(matches)} normalizer(s)")
        return True
    else:
        print(f"  No changes needed")
        return False


def main():
    if len(sys.argv) < 2:
        print("Usage: migrate_normalizers.py <file1.rs> [file2.rs ...]")
        sys.exit(1)

    files_to_process = [Path(f) for f in sys.argv[1:]]

    migrated_count = 0
    for file_path in files_to_process:
        if not file_path.exists():
            print(f"ERROR: File not found: {file_path}")
            continue

        if migrate_normalizer_file(file_path):
            migrated_count += 1

    print(f"\n✓ Successfully migrated {migrated_count} file(s)")


if __name__ == "__main__":
    main()
