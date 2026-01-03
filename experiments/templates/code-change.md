# Code Change: [NAME]

**Date**: YYYY-MM-DD
**Status**: Planning | In Progress | Testing | Completed | Reverted
**Area**: Physics | Morphology | Neural | Chemistry | Integration
**Goal**: [From PLAN.md or user request]

## Objective

**Problem**: What's wrong or missing?

**Hypothesis**: Changing [X] will [expected outcome] because [reasoning].

**Success Criteria**:
- [ ] [Measurable outcome 1]
- [ ] [Measurable outcome 2]

## Changes

### Files to Modify
| File                 | Change        |
|----------------------|---------------|
| `crates/.../file.rs` | [Description] |

### Code Diff
```rust
// Before:
[old code]

// After:
[new code]
```

### Why This Change
[Explain the reasoning behind this specific change]

## Testing

### How to Test
- [ ] `cargo test` passes
- [ ] `cargo clippy` passes
- [ ] Manual observation: [what to look for]
- [ ] Training run: [if applicable]

### Training Command (if applicable)
```bash
just train-quick
# or
cargo run --release --features headless -- \
  --train --scenario locomotion \
  --generations 50 --population 30 \
  --output experiments/YYYY-MM-DD-name
```

## Results

### Before Change
[Describe baseline behavior]

### After Change
[Describe new behavior]

### Metrics (if training)
| Metric       | Before | After | Delta |
|--------------|--------|-------|-------|
| Best fitness |        |       |       |
| Behavior     |        |       |       |

### GIFs
- Before: [path or N/A]
- After: [path]

## Analysis

### Did It Work?
[ ] Yes - hypothesis confirmed
[ ] Partially - some improvement
[ ] No - no change or regression
[ ] Unexpected - different outcome

### What We Learned
-

### Side Effects
-

## Decision

**Keep changes?** Yes | No | Modify further

**Reasoning**:

## Next Steps

If keeping:
1. [ ] Update any related code
2. [ ] Consider follow-up experiments

If reverting:
1. [ ] `git checkout -- [files]`
2. [ ] Document why it didn't work
3. [ ] Try alternative approach

## Artifacts

- Training output: `experiments/YYYY-MM-DD-name/` (if applicable)
- Report: `experiments/reports/YYYY-MM-DD-name.html`
