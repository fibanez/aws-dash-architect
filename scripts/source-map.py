#!/usr/bin/env python3
"""Generate a complete structural map of a Rust source file.

Usage:
    python3 scripts/source-map.py <rust_file_path>

Example:
    python3 scripts/source-map.py src/app/dashui/keyboard_navigation.rs
"""

import re
import sys

def generate_map(file_path):
    """Generate complete map of Rust source file."""

    with open(file_path, 'r') as f:
        content = f.read()

    print('=' * 100)
    print(f'📄 COMPLETE SOURCE MAP: {file_path}')
    print('=' * 100)

    # Extract all enums
    print('\n' + '─' * 100)
    print('📦 ENUMS')
    print('─' * 100)

    enum_pattern = r'(?:///[^\n]*\n)*(?:#\[[^\]]*\]\n)*pub enum (\w+)[^{]*\{([^}]*(?:\{[^}]*\}[^}]*)*)\}'
    for match in re.finditer(enum_pattern, content, re.MULTILINE | re.DOTALL):
        name = match.group(1)
        body = match.group(2)

        line_num = content[:match.start()].count('\n') + 1

        # Extract doc comment
        lines_before = content[:match.start()].split('\n')
        doc = None
        for line in reversed(lines_before[-5:]):
            if line.strip().startswith('///'):
                doc = line.strip()[3:].strip()
                break

        print(f'\n🔹 {name} (line {line_num})')
        if doc:
            print(f'   📝 {doc}')

        # Extract variants
        variants = []
        for line in body.split('\n'):
            line = line.strip()
            if line and not line.startswith('//') and not line.startswith('#['):
                variant_match = re.match(r'([A-Z]\w*)(\([^)]*\)|\{[^}]*\})?', line)
                if variant_match:
                    variant = variant_match.group(1)
                    if variant_match.group(2):
                        variant += variant_match.group(2).replace('\n', '').replace('  ', ' ')
                    variants.append(variant)

        print('   Variants:')
        for v in variants:
            print(f'      • {v}')

    # Extract all structs
    print('\n' + '─' * 100)
    print('🏗️  STRUCTS')
    print('─' * 100)

    struct_pattern = r'(?:///[^\n]*\n)*pub struct (\w+)[^{]*\{([^}]*(?:\{[^}]*\}[^}]*)*?)\}'
    for match in re.finditer(struct_pattern, content, re.MULTILINE | re.DOTALL):
        name = match.group(1)
        body = match.group(2)

        line_num = content[:match.start()].count('\n') + 1

        # Extract doc
        lines_before = content[:match.start()].split('\n')
        doc = None
        for line in reversed(lines_before[-5:]):
            if line.strip().startswith('///'):
                doc = line.strip()[3:].strip()
                break

        print(f'\n🔹 {name} (line {line_num})')
        if doc:
            print(f'   📝 {doc}')

        # Extract fields
        fields = []
        current_field = ''
        for line in body.split('\n'):
            line = line.strip()
            if not line or line.startswith('//') or line.startswith('#['):
                continue

            current_field += ' ' + line
            if ',' in line or '}' in line:
                field_match = re.search(r'(pub\s+)?(\w+)\s*:\s*([^,}]+)', current_field)
                if field_match:
                    visibility = 'pub ' if field_match.group(1) else ''
                    field_name = field_match.group(2)
                    field_type = field_match.group(3).strip()
                    fields.append(f'{visibility}{field_name}: {field_type}')
                current_field = ''

        if fields:
            print('   Fields:')
            for f in fields:
                print(f'      • {f}')

    # Extract all traits
    print('\n' + '─' * 100)
    print('🎯 TRAITS')
    print('─' * 100)

    trait_pattern = r'pub trait (\w+)(?:[^{]*)\{(.*?)\n\}'
    for match in re.finditer(trait_pattern, content, re.MULTILINE | re.DOTALL):
        name = match.group(1)
        body = match.group(2)

        line_num = content[:match.start()].count('\n') + 1

        # Extract doc
        lines_before = content[:match.start()].split('\n')
        doc = None
        for line in reversed(lines_before[-5:]):
            if line.strip().startswith('///'):
                doc = line.strip()[3:].strip()
                break

        print(f'\n🔹 {name} (line {line_num})')
        if doc:
            print(f'   📝 {doc}')

        # Extract method signatures - simplified
        print('   Methods:')
        for line in body.split('\n'):
            line = line.strip()
            if line.startswith('fn '):
                # Extract just fn name and basic signature
                fn_match = re.match(r'fn\s+(\w+)\s*\([^)]*\)(?:\s*->\s*\S+)?', line)
                if fn_match:
                    print(f'      • {fn_match.group(0)};')

    # Extract all impl blocks
    print('\n' + '─' * 100)
    print('⚙️  IMPLEMENTATIONS')
    print('─' * 100)

    impl_pattern = r'impl\s+(?:(\w+)\s+for\s+)?(\w+)\s*\{(.*?)\n\}(?:\s*\n|\s*$)'
    for match in re.finditer(impl_pattern, content, re.MULTILINE | re.DOTALL):
        trait_name = match.group(1)
        type_name = match.group(2)
        body = match.group(3)

        line_num = content[:match.start()].count('\n') + 1

        if trait_name:
            print(f'\n🔹 impl {trait_name} for {type_name} (line {line_num})')
        else:
            print(f'\n🔹 impl {type_name} (line {line_num})')

        # Extract methods - skip tests
        print('   Methods:')
        for line in body.split('\n'):
            line = line.strip()
            if line.startswith('pub fn ') or line.startswith('fn '):
                method_match = re.match(r'(pub\s+)?fn\s+(\w+)\s*\([^)]*\)(?:\s*->\s*\S+)?', line)
                if method_match:
                    method_name = method_match.group(2)
                    if not method_name.startswith('test_'):
                        print(f'      • {method_match.group(0)}')

    print('\n' + '=' * 100)

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python3 scripts/source-map.py <rust_file_path>")
        print("\nExample:")
        print("  python3 scripts/source-map.py src/app/dashui/keyboard_navigation.rs")
        sys.exit(1)

    file_path = sys.argv[1]
    generate_map(file_path)
