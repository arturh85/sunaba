# Iteration: [NAME] (v[N])

**Date**: YYYY-MM-DD
**Status**: Planning | In Progress | Completed | Failed
**Area**: Physics | Morphology | Neural | Chemistry | Integration
**Parent**: [experiments/YYYY-MM-DD-parent.md]
**Chain**: [v1] -> [v2] -> **this**

## Purpose

Build on previous experiment results with a new twist or refinement.

## Background

### Parent Experiment Summary
- **Experiment**: [parent name]
- **Best fitness**: [X]
- **Key finding**: [What worked/didn't work]
- **Suggested next step**: [What parent recommended]

### What This Iteration Changes
[Describe the twist or refinement]

## Objective

**Goal**: [What improvement are you targeting?]

**Hypothesis**: Based on [parent finding], changing [X] should [expected outcome].

**Success Criteria**:
- Outperform parent by >X%
- OR demonstrate new capability
- OR rule out this direction

## Changes from Parent

### Parameter Changes
| Parameter | Parent | This  | Rationale |
|-----------|--------|-------|-----------|
| [param]   | [old]  | [new] | [why]     |

### Code Changes
```rust
// Describe any code modifications
```

### Approach Changes
[Any methodology changes - different scenario, longer training, etc.]

## Command

```bash
cargo run --release --features headless -- \
  --train --scenario locomotion \
  --generations 100 --population 50 \
  --output experiments/YYYY-MM-DD-iteration-vN
```

## Results

### vs Parent
| Metric       | Parent | This | Delta |
|--------------|--------|------|-------|
| Best fitness |        |      |       |
| Avg fitness  |        |      |       |
| Convergence  |        |      |       |
| Behavior     |        |      |       |

### New Observations
[What's different about these creatures?]

### Hypothesis Validated?
[ ] Yes - improvement as expected
[ ] Partially - some improvement but not as expected
[ ] No - no improvement or regression
[ ] Unexpected - different outcome entirely

## Analysis

### What Worked
-

### What Didn't Work
-

### Surprising Results
-

## Chain Progress

### Iteration History
| Version  | Change   | Fitness | Key Learning |
|----------|----------|---------|--------------|
| baseline | Initial  | [X]     | [learning]   |
| v1       | [change] | [X]     | [learning]   |
| v2       | [change] | [X]     | [learning]   |
| **this** | [change] | [X]     | [learning]   |

### Best in Chain
- **Version**: [vN]
- **Fitness**: [X]
- **Why it's best**:

## Next Iteration Ideas

Based on this experiment:
1. [ ] [Next idea with rationale]
2. [ ] [Alternative direction]
3. [ ] [Wild card / exploratory idea]

## Conclusions

**Continue this direction?** Yes / No / Branch
**Reasoning**:

## Artifacts

- Output: `experiments/YYYY-MM-DD-iteration-vN/`
- Champion: `experiments/YYYY-MM-DD-iteration-vN/champion.gif`
- Parent: [link to parent artifacts]
- Report: `experiments/reports/YYYY-MM-DD-iteration-vN.html`
