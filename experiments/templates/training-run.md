# Training Run: [NAME]

**Date**: YYYY-MM-DD
**Status**: Planning | Running | Completed | Failed
**Purpose**: Baseline | Validation | Comparison
**Goal**: [From PLAN.md or user request]

## Objective

**What we're testing**: [Current state / specific changes / comparison]

**Expected outcome**: [What behavior should we see?]

**Success Criteria**:
- [ ] Training completes without errors
- [ ] Champion fitness > [X]
- [ ] [Specific behavior observed]

## Configuration

### Training Parameters
| Parameter     | Value      |
|---------------|------------|
| Scenario      | locomotion |
| Generations   | 100        |
| Population    | 50         |
| Eval duration | 30s        |

### Current Code State
Describe any recent changes being tested, or note "default/baseline".

## Command

```bash
just train-quick              # Fast validation (~1 min)
time just train-quick         # With timing

# Custom scenario/parameters
just train scenario="locomotion" generations="100" population="50"
```

## Results

### Metrics
| Metric            | Value |
|-------------------|-------|
| Best fitness      |       |
| Avg final fitness |       |
| Grid coverage     |       |
| Wall time         |       |

### Champion Behavior
[Describe how the best creature moves]
- Direction:
- Speed:
- Consistency:
- Limb usage:

### Population Overview
- % moving >100px:
- Common behaviors:
- Diverse strategies:

### GIF
View in `training_output/index.html` (embedded in report)

## Analysis

### What Worked
-

### What Didn't Work
-

### Surprising Observations
-

### Comparison to Previous
| Metric       | Previous | This Run | Notes |
|--------------|----------|----------|-------|
| Best fitness |          |          |       |
| Behavior     |          |          |       |

## Conclusions

**Meets success criteria?** Yes | No | Partially

**Key findings**:

## Next Steps

Based on results:
1. [ ] [Next experiment or change]
2. [ ] [Alternative to try]

## Artifacts

- Output: `experiments/YYYY-MM-DD-name/`
- Champion: `experiments/YYYY-MM-DD-name/champion.gif`
- Checkpoints: `experiments/YYYY-MM-DD-name/checkpoints/`
- Report: `experiments/reports/YYYY-MM-DD-name.html`
