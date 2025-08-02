# AWS Dash Test Suite

This test suite is designed to ensure that critical functionality remains frozen and unchanged as the codebase evolves. It uses a chunked testing strategy with smart verbosity levels for optimal context window management.

## Test Categories

### 1. Snapshot Tests (`insta`)
These tests capture the current state of data structures and ensure they don't change unexpectedly:
- `aws_identity_frozen_tests.rs` - Tests AWS credential and account structures
- `projects_frozen_tests.rs` - Tests project and environment structures  
- `cfn_dag_frozen_tests.rs` - Tests CloudFormation DAG structures

### 2. Golden File Tests
These tests compare output against stored "golden" files:
- Configuration formats
- API response formats
- Complex data structures

### 3. Contract Tests
`api_contract_simple_tests.rs` ensures the public API remains stable by:
- Verifying method signatures exist
- Testing struct field accessibility
- Ensuring trait implementations
- Checking enum variants

## Running Tests

### Recommended: Chunked Testing with Smart Verbosity

**For development and debugging** (smart mode - shows failures without flooding):
```bash
./scripts/test-chunks.sh fast           # All essential tests (~126 tests)
./scripts/test-chunks.sh core           # Core functionality tests (frozen: 10, API: 8, unit: 108)
./scripts/test-chunks.sh cfn            # CloudFormation logic tests
./scripts/test-chunks.sh ui             # UI component tests (REMOVED - see note below)
./scripts/test-chunks.sh projects       # Project management tests
```

**For different verbosity levels:**
```bash
./scripts/test-chunks.sh core                    # Smart mode (default)
TEST_MODE=quiet ./scripts/test-chunks.sh core    # Minimal output
TEST_MODE=detailed ./scripts/test-chunks.sh core # Debugging details
TEST_MODE=full ./scripts/test-chunks.sh core     # Complete output
```

**For comprehensive testing:**
```bash
./scripts/test-chunks.sh all            # All tests including integration (~30min)
```

### ⚠️ UI Testing Status (August 2025)

**UI component tests have been REMOVED** due to compilation and import issues. The `ui` chunk currently shows a skip message instead of running tests. For complete details about the removed UI tests and potential restoration approaches, see `tests/UI_TESTING_SETUP.md`.

**What was removed:**
- ~20 UI test files covering basic components, CloudFormation forms, window management
- ~100-150 individual UI tests for interaction patterns and visual consistency
- egui_kittest-based testing infrastructure for UI automation

**Current alternatives:**
- Manual testing procedures for UI workflows
- Integration tests focus on business logic without UI layer
- Existing window focus system tests remain functional

**For UI testing documentation:**
- See `tests/UI_TESTING_SETUP.md` for complete archive of removed tests
- Contains restoration notes and alternative testing approaches
- Documents egui_kittest integration patterns that were working

### Benefits of Chunked Testing

- **Context window friendly**: Smart mode provides essential failure info without flooding
- **Performance optimized**: Organized by execution time and scope
- **AI assistant compatible**: Perfect balance of information for troubleshooting
- **Incremental testing**: Run only the test categories you need
- **Backwards compatible**: All existing VERBOSE settings still work

### Traditional Cargo Commands

```bash
# Run all tests (large context, not recommended for development)
cargo test

# Run with snapshot review
cargo insta test
cargo insta review

# Run specific test module
cargo test aws_identity_frozen_tests

# Update golden files (be careful!)
cargo test -- --ignored update_golden_files
```

## Adding New Tests

When adding new functionality:

1. **Add snapshot tests** for new data structures:
   ```rust
   #[test]
   fn test_new_structure() {
       let data = MyNewStruct { ... };
       assert_json_snapshot!("new_struct", data);
   }
   ```

2. **Add golden file tests** for file formats:
   ```rust
   #[test]
   fn test_file_format() {
       let content = generate_content();
       let expected = read_golden_file("expected_format.txt");
       assert_eq!(content, expected);
   }
   ```

3. **Update contract tests** for API changes:
   ```rust
   #[test]
   fn test_new_api_contract() {
       let instance = NewApi::new();
       let _result = instance.critical_method();
   }
   ```

## Snapshot Management

Snapshots are stored in `tests/snapshots/` and are managed by `insta`:

- Review changes: `cargo insta review`
- Accept changes: `cargo insta accept`
- Reject changes: `cargo insta reject`

## Golden Files

Golden files are stored in `tests/fixtures/` and represent expected outputs:

- Update carefully when output format changes are intentional
- Always review diffs before committing
- Document why changes were made

## CI Integration

The test suite should be run in CI to catch any unintended changes:

### Recommended CI Configuration

```yaml
- name: Fast Test Suite
  run: |
    ./scripts/test-chunks.sh fast
    cargo insta test --unreferenced=reject

- name: Full Test Suite (optional)
  run: |
    ./scripts/test-chunks.sh all
```

### Legacy CI Configuration

```yaml
- name: Run tests (not recommended - large context)
  run: |
    cargo test
    cargo insta test --unreferenced=reject
```

## Best Practices

1. **Never blindly accept snapshot changes** - Always review what changed and why
2. **Document breaking changes** - If a test must be updated, explain why in the commit
3. **Test edge cases** - Include tests for error conditions and boundary values
4. **Keep tests focused** - Each test should verify one specific aspect
5. **Use descriptive names** - Test names should clearly indicate what they're testing

## Troubleshooting

### Snapshot Mismatches
- Run `cargo insta test` to see the differences
- Use `cargo insta review` to interactively accept/reject changes

### Golden File Failures
- Check `tests/fixtures/` for the expected content
- Diff the actual vs expected output
- Update the golden file only if the change is intentional

### Contract Test Failures
- These indicate breaking API changes
- Consider if the change is necessary
- Update documentation if proceeding with the change