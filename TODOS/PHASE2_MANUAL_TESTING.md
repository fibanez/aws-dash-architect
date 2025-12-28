# Phase 2 Enrichment - Manual Testing Guide

This guide walks you through testing the Two-Phase Resource Loading feature. Follow each test step-by-step.

---

## Before You Start

### What You Need
1. The AWS Dash application running (`cargo run`)
2. At least one AWS account configured in AWS Identity Center
3. A bookmark that includes S3 buckets OR Lambda functions (these are "enrichable" resources)

### Key Concepts
- **Phase 1**: Fast query that gets basic resource info (name, ID, region)
- **Phase 2**: Slower query that gets detailed info (policies, encryption, configurations)
- **Enrichable Resources**: S3, Lambda, IAM, KMS, SQS, SNS, DynamoDB, etc.
- **Non-Enrichable Resources**: EC2 instances, VPCs, Subnets (already have full details in Phase 1)

---

## Test 1: Phase 2 Triggers Automatically

**Goal**: Verify that Phase 2 starts automatically after Phase 1 completes.

### Steps

1. **Open the Resource Explorer window**
   - Look for "Resource Explorer" in the application menu or window list

2. **Click on a bookmark that has S3 buckets or Lambda functions**
   - The bookmark name will appear in the Explorer

3. **Watch the status bar at the bottom of the Explorer window**
   - You should see it go through these stages:
     - First: "Loading..." (Phase 1 is running)
     - Then: Resources appear in the tree
     - Then: "Loading details... (1/N)" (Phase 2 has started)

### What Success Looks Like
- [X] Status bar shows "Loading details... (X/Y)" after resources appear
- [X] The numbers (X/Y) increase over time (1/10, 2/10, 3/10...)

### What Failure Looks Like
- Status bar never shows "Loading details..."
- Resources appear but no Phase 2 progress is shown

---

## Test 2: Status Bar Shows Progress

**Goal**: Verify the status bar updates as each resource is enriched.

### Steps

1. **Load a bookmark with at least 5 enrichable resources** (S3 buckets or Lambda functions)

2. **Watch the status bar closely**
   - It should show: "Loading details... (X/Y)"
   - The X number should increase: 1, 2, 3, 4, 5...
   - The Y number is the total count of enrichable resources

3. **Time the updates**
   - Updates should happen every few seconds (not all at once)
   - Each update means one more resource got its detailed info

### What Success Looks Like
- [X] Status bar shows "Loading details... (X/Y)"
- [X] X increases over time (not jumping from 0 to final number)
- [X] When X equals Y, the status bar clears (Phase 2 complete)

### What Failure Looks Like
- X jumps from 0 to a large number (missing intermediate updates)
- Status bar gets stuck at a number and never completes
- Status bar never appears at all

---

## Test 3: Loading Message Inside Resource Node

**Goal**: Verify that when you expand an enrichable resource during Phase 2, it shows "Loading details..."

### Steps

1. **Load a bookmark with S3 buckets or Lambda functions**

2. **As soon as you see "Loading details... (1/N)" in the status bar:**
   - Quickly click to expand one of the S3 bucket or Lambda nodes in the tree
   - You need to do this BEFORE Phase 2 completes

3. **Look inside the expanded node**
   - You should see: "Loading details..." in gray italic text
   - There should be a small spinning indicator

4. **Wait for Phase 2 to complete**
   - The "Loading details..." message should disappear
   - The detailed properties should appear instead

### What Success Looks Like
- [X] "Loading details..." appears inside the node during Phase 2
- [X] Message disappears when Phase 2 completes
- [X] Detailed properties (like policies) appear after Phase 2

### What Failure Looks Like
- No "Loading details..." message appears inside the node
- Message appears but never goes away
- Detailed properties never appear

---

## Test 4: Non-Enrichable Resources Don't Show Loading

**Goal**: Verify that EC2 instances, VPCs, and other non-enrichable resources don't show "Loading details..."

### Steps

1. **Load a bookmark that includes EC2 instances or VPCs**
   - These are "non-enrichable" - they get all their data in Phase 1

2. **Expand an EC2 instance or VPC node in the tree**

3. **Check for loading message**
   - You should NOT see "Loading details..." inside the node
   - The properties should appear immediately

### What Success Looks Like
- [X] No "Loading details..." message for EC2/VPC resources
- [X] Properties appear immediately without waiting

### What Failure Looks Like
- "Loading details..." appears for EC2 or VPC resources (incorrect)

---

## Test 5: Tree Updates When Phase 2 Completes

**Goal**: Verify that the tree shows enriched data without needing to click or interact.

### Steps

1. **Load a bookmark with S3 buckets**

2. **Expand one of the S3 bucket nodes BEFORE Phase 2 starts or during Phase 2**
   - Note what properties are shown

3. **Leave the node expanded and wait for Phase 2 to complete**
   - Watch the status bar until it clears

4. **Check the expanded node again (WITHOUT clicking anything)**
   - New properties should appear automatically
   - Look for "DetailedInfo" section with policies, encryption settings, etc.

### What Success Looks Like
- [X] New properties appear automatically without clicking
- [X] "DetailedInfo" section visible after Phase 2 completes

### What Failure Looks Like
- Node content doesn't change after Phase 2
- You have to click away and back to see new data

---

## Test 6: Phase 2 Completes Properly (No Stuck Progress)

**Goal**: Verify that Phase 2 completes and doesn't get stuck at N-1/N.

### Steps

1. **Load a bookmark with enrichable resources**

2. **Watch the status bar progress**
   - Note the total number (the Y in X/Y)

3. **Wait for Phase 2 to complete**
   - The progress should reach Y/Y (e.g., 22/22)
   - Then the status bar should clear

4. **Verify completion**
   - Status bar should no longer show "Loading details..."
   - The Explorer should be in a "ready" state

### What Success Looks Like
- [X] Progress reaches the final number (22/22 not 21/22)
- [X] Status bar clears after completion
- [X] No lingering "Loading details..." message

### What Failure Looks Like
- Progress gets stuck at N-1/N (like 21/22) and never completes
- Status bar never clears

---

## Test 7: Loading New Bookmark Resets Phase 2 Status

**Goal**: Verify that switching bookmarks clears any stuck Phase 2 state.

### Steps

1. **Load Bookmark A** (one with enrichable resources)
   - Wait for Phase 2 to complete

2. **Load Bookmark B** (a different bookmark)
   - Watch the status bar

3. **Check that:**
   - No leftover progress from Bookmark A is shown
   - If Bookmark B has enrichable resources, Phase 2 starts fresh

4. **Go back to Bookmark A**
   - Progress should NOT show old numbers from before
   - If data is cached, Phase 2 might not run again (that's OK)

### What Success Looks Like
- [X] Switching bookmarks clears old Phase 2 progress
- [X] No "stuck" progress numbers from previous bookmark
- [X] Each bookmark starts with fresh state

### What Failure Looks Like
- Old progress numbers (like 21/22) appear when loading new bookmark
- Phase 2 status from Bookmark A shows when viewing Bookmark B

---

## Test 8: Agent Query with detail="full" Waits for Phase 2

**Goal**: Verify that an AI agent requesting full details waits for Phase 2.

### Prerequisites
- You need access to the Agent window/tool in AWS Dash
- Or a way to run JavaScript queries directly

### Steps

1. **Open an Agent chat or JavaScript console**

2. **Run a query with detail="full":**
   ```javascript
   const result = queryResources({
     accounts: null,  // use any account
     regions: ["us-east-1"],
     resourceTypes: ["AWS::S3::Bucket"],
     detail: "full"
   });
   console.log(result);
   ```

3. **Check the result:**
   - Look for `detailsLoaded: true` in the response
   - Look for `detailedProperties` on the resources (not null)

4. **Check timing:**
   - If there are many S3 buckets, the query should take several seconds
   - This is because it's waiting for Phase 2 to complete

### What Success Looks Like
- [ ] Query returns after Phase 2 completes (not immediately)
- [ ] `detailsLoaded: true` in response
- [ ] Resources have `detailedProperties` filled in

### What Failure Looks Like
- Query returns immediately with `detailedProperties: null`
- `detailsLoaded: false` even though resources support enrichment

---

## Test 9: Agent Query with detail="summary" Returns Immediately

**Goal**: Verify that summary queries don't wait for Phase 2.

### Steps

1. **Open an Agent chat or JavaScript console**

2. **Run a query with detail="summary":**
   ```javascript
   const result = queryResources({
     accounts: null,
     regions: ["us-east-1"],
     resourceTypes: ["AWS::S3::Bucket"],
     detail: "summary"
   });
   console.log(result);
   ```

3. **Check timing:**
   - The query should return quickly (within a few seconds)
   - It should NOT wait for Phase 2

4. **Check the result:**
   - Resources should have basic info (id, name, type)
   - `detailedProperties` may be null (that's expected)

### What Success Looks Like
- [ ] Query returns quickly (doesn't wait for Phase 2)
- [ ] Basic resource info is present
- [ ] Response may show `detailsPending: true` if Phase 2 is running

### What Failure Looks Like
- Query takes a long time (incorrectly waiting for Phase 2)

---

## Test 10: Agent Response Includes Phase 2 Metadata

**Goal**: Verify that agent responses include `detailsLoaded` and `detailsPending` fields.

### Steps

1. **Run a query during Phase 2:**
   - Load a large bookmark in the Explorer first
   - While "Loading details..." is showing, run an agent query

2. **Check the response structure:**
   ```javascript
   const result = queryResources({
     resourceTypes: ["AWS::Lambda::Function"],
     detail: "summary"
   });

   console.log("detailsLoaded:", result.detailsLoaded);
   console.log("detailsPending:", result.detailsPending);
   ```

3. **Expected values during Phase 2:**
   - `detailsLoaded: false` (not all details loaded yet)
   - `detailsPending: true` (background loading in progress)

4. **Expected values after Phase 2:**
   - `detailsLoaded: true` (details available)
   - `detailsPending: false` (no more loading)

### What Success Looks Like
- [ ] Response includes `detailsLoaded` field
- [ ] Response includes `detailsPending` field
- [ ] Values correctly reflect Phase 2 state

### What Failure Looks Like
- Fields are missing from response
- Fields always show false regardless of Phase 2 state

---

## Summary Checklist

After completing all tests, verify:

| Test | Description | Pass/Fail |
|------|-------------|-----------|
| 1 | Phase 2 triggers automatically | |
| 2 | Status bar shows progress updates | |
| 3 | "Loading details..." in resource node | |
| 4 | Non-enrichable resources don't show loading | |
| 5 | Tree updates automatically after Phase 2 | |
| 6 | Phase 2 completes (no stuck progress) | |
| 7 | New bookmark resets Phase 2 status | |
| 8 | Agent detail="full" waits for Phase 2 | |
| 9 | Agent detail="summary" returns immediately | |
| 10 | Agent response includes Phase 2 metadata | |

---

## Troubleshooting

### If tests fail, check the log file:
```bash
tail -100 ~/.local/share/awsdash/logs/awsdash.log
```

### Look for these log messages:
- `Phase 2 enrichment completed` - Confirms Phase 2 finished
- `Phase 2: Updated cache for` - Shows individual resource updates
- `Tree cache miss - rebuilding` - Shows tree is refreshing

### Common issues:
1. **No enrichable resources**: Make sure your bookmark includes S3, Lambda, or other enrichable types
2. **Cached data**: If you've run the same query before, data may already be enriched (try a new bookmark)
3. **Lock contention**: If progress gets stuck, this is a bug - note the numbers and report it

---

## Notes for Tester

- Take screenshots of any failures
- Note the exact numbers if progress gets stuck (e.g., "stuck at 21/22")
- Record how long Phase 2 takes for different numbers of resources
- Note any console errors or log messages that seem relevant
