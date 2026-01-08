# Architecture: System Design

This page provides technical diagrams showing how Sunaba's systems fit together. For code-level details, see the repository's CLAUDE.md file.

## High-Level System Overview

```mermaid
flowchart TB
    subgraph "Game Layer"
        UI[User Interface]
        R[Renderer]
        Input[Input Handler]
    end

    subgraph "Simulation Core"
        W[World Manager]
        CA[Cellular Automata Engine]
        T[Temperature System]
        S[Structural Integrity]
        P[Physics Engine]
    end

    subgraph "Creature System"
        C[Creature Manager]
        B[Brain/Neural Controller]
        G[GOAP Planner]
        Sensors[Sensor System]
    end

    subgraph "Evolution Pipeline"
        ME[MAP-Elites Archive]
        CPPN[CPPN-NEAT Generator]
        Eval[Fitness Evaluation]
    end

    Input --> W
    W --> CA
    W --> T
    W --> S
    W --> P
    CA --> R
    P --> C
    Sensors --> B
    B --> C
    C --> W
    CPPN --> C
    Eval --> ME
    ME --> CPPN
```

## Simulation Layers

The world simulation operates in layers with different update frequencies:

```mermaid
flowchart TD
    subgraph "60 FPS"
        CA[Cellular Automata]
        Physics[Rigid Body Physics]
        Neural[Neural Inference]
    end

    subgraph "30 FPS"
        Temp[Temperature Diffusion]
        GOAP[GOAP Planning]
    end

    subgraph "Event-Driven"
        Struct[Structural Integrity]
        React[Chemical Reactions]
    end

    CA --> Temp
    CA --> Struct
    Physics --> Neural
    Neural --> GOAP
```

### Layer Interaction

```mermaid
sequenceDiagram
    participant CA as Cellular Automata
    participant Temp as Temperature
    participant Struct as Structural
    participant Phys as Physics

    loop Every Frame
        CA->>CA: Update pixel positions
        CA->>Temp: Report heat sources
        Temp->>Temp: Diffuse heat (every 2nd frame)
        Temp->>CA: Trigger state changes
        CA->>Struct: Report removals
        Struct->>Phys: Create debris bodies
        Phys->>CA: Update body positions
    end
```

## World Structure

### Chunk Organization

```mermaid
flowchart LR
    subgraph "World"
        direction TB
        subgraph "Active Chunks"
            A1[Chunk 0,0]
            A2[Chunk 1,0]
            A3[Chunk 0,1]
            A4[Chunk 1,1]
        end

        subgraph "Loaded Chunks"
            L1[Chunk -1,0]
            L2[Chunk 2,0]
        end

        subgraph "Disk"
            D1[Serialized chunks...]
        end
    end

    A1 <--> A2
    A2 <--> A4
    A1 <--> A3
    A3 <--> A4
    L1 <--> A1
    L2 <--> A2
    D1 -.-> L1
    D1 -.-> L2
```

### Chunk Data Layout

```
Chunk (64×64 pixels)
├── pixel_data: [u32; 4096]
│   ├── bits 0-15:  material_id
│   ├── bits 16-23: flags
│   └── bits 24-31: variant/metadata
├── temperature: [f32; 64]  (8×8 coarse grid)
├── light: [u8; 4096]
└── dirty_rect: Option<Rect>
```

## Evolution Pipeline

### Offline Training Flow

```mermaid
flowchart TD
    subgraph "Initialization"
        I1[Create random CPPN genomes]
        I2[Initialize MAP-Elites archive]
    end

    subgraph "Evolution Loop"
        S1[Select from archive]
        S2[Mutate genome]
        S3[Build morphology from CPPN]
        S4[Initialize neural controller]
        S5[Run evaluation scenario]
        S6[Compute fitness + behavior]
        S7[Update archive]
    end

    I1 --> I2
    I2 --> S1
    S1 --> S2
    S2 --> S3
    S3 --> S4
    S4 --> S5
    S5 --> S6
    S6 --> S7
    S7 --> S1
```

### CPPN Morphology Generation

```mermaid
flowchart TD
    subgraph "CPPN Query"
        Q1[Define bounding box]
        Q2[Generate query points]
        Q3[For each point x,y,d]
        Q4[Feed into CPPN network]
        Q5[Get outputs]
    end

    subgraph "Body Construction"
        B1{presence > threshold?}
        B2[Create segment]
        B3[Assign to body graph]
        B4[Determine joint type]
        B5[Connect to neighbors]
    end

    Q1 --> Q2 --> Q3 --> Q4 --> Q5
    Q5 --> B1
    B1 -->|Yes| B2
    B2 --> B3 --> B4 --> B5
    B1 -->|No| Q3
    B5 --> Q3
```

## Creature Lifecycle

```mermaid
stateDiagram-v2
    [*] --> Spawning: CPPN generates body

    Spawning --> Active: Body valid

    state Active {
        [*] --> Sensing
        Sensing --> Thinking: Sensor data collected
        Thinking --> Acting: Motor commands generated
        Acting --> Sensing: Physics stepped
    }

    Active --> Dead: Health <= 0
    Active --> Dead: Fall out of world
    Active --> Reproducing: Fitness threshold

    Reproducing --> Active: Offspring spawned

    Dead --> [*]: Despawn
```

## Neural Control Pipeline

### Per-Frame Processing

```mermaid
flowchart LR
    subgraph "Sensors"
        P[Proprioception]
        E[Exteroception]
        I[Internal State]
    end

    subgraph "Encoding"
        PE[Prop Encoder]
        EE[Ext Encoder]
        IE[State Encoder]
    end

    subgraph "Brain"
        H1[Hidden 64]
        H2[Hidden 64]
        H3[Hidden 32]
        O[Output]
    end

    subgraph "Motors"
        M1[Joint 1]
        M2[Joint 2]
        M3[Joint N]
    end

    P --> PE --> H1
    E --> EE --> H1
    I --> IE --> H1
    H1 --> H2 --> H3 --> O
    O --> M1
    O --> M2
    O --> M3
```

### Sensor Details

```mermaid
flowchart TD
    subgraph "Proprioception per Joint"
        J1[angle: -π to π]
        J2[velocity: normalized]
        J3[torque: normalized]
        J4[contact: 0 or 1]
    end

    subgraph "Exteroception"
        R1[8 raycasts × 3 values]
        R2[distance, material, temperature]
    end

    subgraph "Internal State"
        S1[health: 0 to 1]
        S2[hunger: 0 to 1]
        S3[orientation: sin, cos]
    end
```

## Training Scenarios

### Scenario Types

```mermaid
flowchart LR
    subgraph "Locomotion"
        L1[Flat terrain]
        L2[Target point reward]
        L3[Time limit: 30s]
    end

    subgraph "Foraging"
        F1[Scattered food]
        F2[Consumption reward]
        F3[Time limit: 60s]
    end

    subgraph "Survival"
        S1[Hazards present]
        S2[Food + threats]
        S3[Time limit: 120s]
    end

    subgraph "Combat"
        C1[Opponent creature]
        C2[Damage dealt reward]
        C3[Time limit: 60s]
    end
```

### Fitness Computation

```mermaid
flowchart TD
    subgraph "Raw Metrics"
        M1[distance_traveled]
        M2[food_eaten]
        M3[time_survived]
        M4[damage_dealt]
        M5[offspring_produced]
    end

    subgraph "Weights"
        W1[scenario-specific weights]
    end

    subgraph "Final Fitness"
        F[weighted sum]
    end

    M1 --> W1
    M2 --> W1
    M3 --> W1
    M4 --> W1
    M5 --> W1
    W1 --> F
```

## MAP-Elites Archive

### Archive Structure

```mermaid
flowchart TD
    subgraph "2D Behavior Space"
        direction LR
        subgraph "Axis 1: Ground Contact"
            X1[0.0]
            X2[0.25]
            X3[0.5]
            X4[0.75]
            X5[1.0]
        end
    end

    subgraph "Archive Grid"
        G[20 × 20 cells]
        E[Each cell: elite genome + fitness]
    end

    X1 --> G
    X5 --> G
```

### Archive Operations

```mermaid
sequenceDiagram
    participant E as Evaluator
    participant A as Archive
    participant S as Selector

    E->>E: Evaluate creature
    E->>E: Compute behavior descriptor
    E->>A: Check cell for behavior
    alt Cell empty
        A->>A: Store genome + fitness
    else Cell occupied
        alt New fitness > stored fitness
            A->>A: Replace with new
        else
            A->>A: Discard new
        end
    end
    S->>A: Request random elite
    A->>S: Return genome for mutation
```

## Data Flow Summary

```mermaid
flowchart TB
    subgraph "Genotype"
        CPPN_G[CPPN Genome]
        NN_G[Neural Weights]
    end

    subgraph "Phenotype"
        Body[Physical Body]
        Brain[Neural Controller]
    end

    subgraph "Behavior"
        Actions[Motor Commands]
        Interact[World Interactions]
    end

    subgraph "Evaluation"
        Metrics[Performance Metrics]
        BC[Behavior Characterization]
        Fit[Fitness Score]
    end

    subgraph "Selection"
        Archive[MAP-Elites Archive]
        Mutation[Mutation Operators]
    end

    CPPN_G --> Body
    NN_G --> Brain
    Body --> Actions
    Brain --> Actions
    Actions --> Interact
    Interact --> Metrics
    Metrics --> BC
    Metrics --> Fit
    BC --> Archive
    Fit --> Archive
    Archive --> Mutation
    Mutation --> CPPN_G
    Mutation --> NN_G
```

## User Interface

Sunaba's UI is built with egui, providing immediate-mode rendering for game controls and information displays.

### Inventory System

![Inventory Panel](/screenshots/ui_inventory.png)

The inventory system supports:
- **Item Stacking**: Materials stack up to 999 per slot
- **Tool Durability**: Tools show remaining uses and degrade with use
- **Quick Access**: Hotbar for frequently used items
- **Visual Feedback**: Item icons with quantity displays

### Crafting System

![Crafting Panel](/screenshots/ui_crafting.png)

The crafting interface provides:
- **Recipe Discovery**: Available recipes shown based on inventory
- **Material Requirements**: Clear display of required ingredients
- **Output Preview**: Shows crafted item before creation
- **Batch Crafting**: Support for creating multiple items at once

### Material Showcase

![Material Catalog](/screenshots/level_3.png)
*Basic material showcase: 8 fundamental materials with distinct visual properties*

![Extended Materials](/screenshots/level_17.png)
*Phase 5 materials: 30+ materials including ores, organics, refined materials, and special compounds*

### Lighting & World Systems

![Day/Night Cycle](/screenshots/level_20.png)
*Light propagation demonstration: underground lava, fire sources, and surface daylight*

### Structural Complexity

![Castle Structure](/screenshots/level_12.png)
*Complex architectural structures demonstrating building capabilities*

![Survival Environment](/screenshots/level_16.png)
*Tutorial level with resource distribution and starter area*

## Tech Stack Summary

| Component | Technology |
|-----------|------------|
| Language | Rust |
| Graphics | wgpu |
| UI | egui |
| Windowing | winit |
| Physics | rapier2d |
| Math | glam |
| Serialization | serde + bincode |
| Compression | lz4_flex |
| RNG | rand + rand_xoshiro |
| Graphs | petgraph |
