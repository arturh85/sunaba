---
name: research
description: Run structured research experiments for Sunaba creature evolution. Use for creating experiments, investigating problems, making code changes, running training, analyzing results, and tracking progress across physics, morphology, neural, chemistry, and integration work.
allowed-tools: Read, Grep, Glob, Write, Edit, Bash(cargo run:*), Bash(just:*), Bash(time:*), Bash(RUST_LOG=*:*), Bash(cargo test:*), Bash(cargo build:*)
---

# Research Experiments Skill

Systematic research for Sunaba creature evolution across all systems.

## Research Areas

| Area        | What Changes                               | How to Test                       |
|-------------|--------------------------------------------|-----------------------------------|
| Physics     | Gravity, damping, friction, thrust, joints | Training runs, observe behavior   |
| Morphology  | Body generation, part sizes, joint limits  | Training runs, visual inspection  |
| Neural      | Network architecture, sensors, inputs      | Training runs, behavior analysis  |
| Chemistry   | Reactions, materials, world generation     | Manual testing, emergent behavior |
| Integration | How systems work together                  | In-game testing with creatures    |

## Commands

### `/research new <goal>`
Create a new experiment from a goal description.

**What I'll do:**
1. Parse the goal (often from PLAN.md)
2. Determine research area and appropriate template
3. Generate descriptive experiment name
4. Create `experiments/YYYY-MM-DD-<name>.md`
5. Add entry to `experiments/PROGRESS.md`

**Examples:**
```
/research new "creatures should move >100px consistently"
/research new "investigate why creatures don't use their legs"
/research new "test higher joint angular velocity"
```

### `/research suggest`
Propose experiments based on current goals and progress.

**What I'll do:**
1. Read `PLAN.md` for current phase goals
2. Read `experiments/PROGRESS.md` for completed work
3. Analyze what hasn't been tried yet
4. Propose 2-5 experiments with:
   - Goal and hypothesis
   - Research area (physics/morphology/neural/etc.)
   - What we expect to learn
   - Estimated effort
5. Ask which ones to create

### `/research resume <experiment-file>`
Continue work on an existing experiment.

**What I'll do:**
1. Read the experiment file and current status
2. Offer options based on status:
   - **Failed/Incomplete**: Retry or adjust approach
   - **Completed**: Create iteration with new twist
   - **Any**: Analyze current state
3. If iterating, create new experiment linked to original

**Use cases:**
- Training crashed or timed out
- New idea to try on same problem
- Build iteration chain (v1 -> v2 -> v3)

### `/research status`
Show overview of all experiments.

**What I'll do:**
1. Read `experiments/PROGRESS.md`
2. List experiments by status
3. Show iteration chains
4. Highlight current phase goals from PLAN.md

### `/research analyze <experiment-file>`
Analyze a completed experiment.

**What I'll do:**
1. Read experiment file and any training output
2. Check for logs, checkpoints
3. Summarize findings
4. Compare to baseline or previous iteration
5. Suggest next steps

### `/research report <experiment-file>`
Suggest improvements to training reports.

**What I'll do:**
1. Read experiment file and `training_output/` results
2. Analyze what visualizations would help understand findings
3. Propose specific additions to `report.rs` or new standalone reports
4. Implement approved report improvements

## Templates

Choose based on experiment type:

| Template           | Use When                                                        |
|--------------------|-----------------------------------------------------------------|
| `code-change.md`   | Modifying game systems (physics, chemistry, morphology, neural) |
| `training-run.md`  | Running training to test changes or establish baseline          |
| `investigation.md` | Understanding a problem before trying fixes                     |
| `iteration.md`     | Building on previous experiment results                         |

## Workflow

### Investigation Flow
1. `/research new "understand why X happens"`
2. Read code, run tests, observe behavior
3. Document findings in experiment file
4. Propose fixes as new experiments

### Code Change Flow
1. `/research new "try changing X to improve Y"`
2. Make code changes
3. Run training or manual tests
4. Document results
5. Keep or revert changes

### Training Flow
1. Make code changes (or use current state)
2. `/research new "establish baseline for X"`
3. Run `just train-quick` or full training
4. Analyze results with `/research analyze`
5. Iterate with `/research resume`

### Iteration Flow
1. Complete an experiment
2. `/research resume experiments/prev-exp.md`
3. Choose "iterate with new twist"
4. New experiment links to parent
5. Track chain in PROGRESS.md

## Report Improvements

When conducting research, consider what visualizations or metrics would help understanding. Propose additions to existing reports or entirely new reports.

**Existing report infrastructure** (`crates/sunaba/src/headless/report.rs`):
- HTML reports with embedded GIFs, fitness charts, MAP-Elites grid
- `summary.json` with machine-readable metrics
- Output location: `training_output/`

**Example suggestions by research area**:

| Area | Potential Report Improvements |
|------|------------------------------|
| Physics | Joint force heatmaps, momentum graphs, ground contact timeline |
| Morphology | Body structure diagrams, limb length distributions, symmetry metrics |
| Neural | Network activation heatmaps, sensor input plots, decision timeline |
| Fitness | Per-component fitness breakdown, generation-by-generation behavior changes |
| Integration | Combined system interaction diagrams |

**When to suggest**:
- After completing a training run, propose visualizations that would clarify findings
- When investigation reveals patterns that are hard to describe textually
- When comparing iterations, propose comparison visualizations

## Reference

### Training Commands
```bash
just train-quick              # Fast validation (~1 min), outputs to training_output/
time just train-quick         # With timing

# Custom scenario/parameters (still uses training_output/)
just train scenario="locomotion" generations="100" population="50"
```

### Scenarios
locomotion, simple, foraging, survival, balanced, parcour

### Key Files
| Area       | Key Files                                          |
|------------|----------------------------------------------------|
| Physics    | `creature.rs`, `physics.rs`, `morphology.rs`       |
| Morphology | `morphology.rs`, `genome.rs`                       |
| Neural     | `neural.rs`, `sensors.rs`                          |
| Training   | `headless/training_env.rs`, `headless/scenario.rs` |
| Fitness    | `headless/fitness.rs`                              |
