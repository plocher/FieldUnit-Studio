# FieldUnit Studio: Model Board Layout Engine and Virtual cTc Architecture

## 1. Architectural Overview

```
                      ( Cloud: Designer / Operator )
                                    │
                                    │ User Actions
                                    ▼
       ( Cloud: Schematic View )        ( Cloud: Virtual cTc Desk )
       - Engineering detail             - Compressed model board
       - IRJs, circuits, pins           - Lever columns & code buttons
               │                                    │
               │ Topology Edits                     │ Controls-as-Demands
               ▼                                    ▼
┌───────────────────────────────────────────────────────────────────┐
│               ( Cloud: Station Corridor Matrix )                  │
│       - Corridors: Discrete track levels (Main, Siding, Spur)     │
│       - Station Columns: Linear X-slots (aligned to CTC plates)   │
│       - Geometric Constraints: 0°, 180°, ±45° links               │
└─────────────────────────────────┬─────────────────────────────────┘
                                  │
                                  ▼
┌───────────────────────────────────────────────────────────────────┐
│                  ( Cloud: Core Domain Engine )                    │
│       - Route Synthesizer (Path traversal, speed ceilings)        │
│       - Design Rule Checker (Connectivity, electrical gaps)       │
│       - Interface "A" Codec (Snapshots & Indications)             │
└─────────────────────────────────┬─────────────────────────────────┘
                                  │
                                  │ Interface "A" Byte Vectors
                                  ▼
       ( Cloud: Simulation Engine )     ( Cloud: Physical Layout )
       - In-memory mock plant           - USB / RS-485 / MQTT
       - Point transit delay            - FieldUnit microcontrollers
```

---

## 2. The Core Problem

Previous iterations treated schematic symbols as free-floating Cartesian $(x, y)$ pixels. That design had fatal flaws:
1. Moving one component allowed tracks to drift into non-standard angles.
2. Inserting new items required arbitrary coordinate math that corrupted adjacent tracks.
3. Rotating a switch flipped pixels locally without updating the attached route.
4. The canvas had no concept of railroad corridors or vertical CTC lever columns.

---

## 3. The Station Corridor Layout Model

The layout engine replaces floating coordinates with a discrete topological matrix:

### 3.1 Corridors (Y-Levels)
Tracks occupy discrete horizontal levels:
- `Level 0`: Main Line 1
- `Level 1`: Passing Siding / Main Line 2
- `Level -1` / `Level 2`: Industrial Spurs or Pocket Tracks

The distance between corridors is fixed (for example, 100 pixels).

### 3.2 Station Columns (X-Slots)
The layout divides the horizontal axis into ordered station columns:
$$\text{Station } S_0, S_1, S_2, \dots, S_n$$

Every column represents a physical vertical slice of the railroad:
- On the **Upper Model Board**: The column displays the switch points, IRJs, and signal lamps for that location.
- On the **Lower cTc Plate**: The column aligns with the physical switch lever, signal lever, and code button for that station.

### 3.3 Appliances as Grid Relationships
- **Tangent Track**: A horizontal link between adjacent slots on the same corridor:
  $$(S_i, \text{Level } L) \longleftrightarrow (S_{i+1}, \text{Level } L)$$
- **Turnout**: An appliance at $(S_i, \text{Level } L)$ with:
  - Normal leg continuing horizontally on $\text{Level } L$.
  - Reverse leg linking to an adjacent corridor $(S_{i+1}, \text{Level } L \pm 1)$ at an exact $45^\circ$ angle.

---

## 4. Formal Transformation Operations

The Layout Engine enforces strict geometric transformations:

### 4.1 Rotate (`R`)
- Rotates the facing direction along the track axis by $180^\circ$.
- **Facing East** (points face left, diverge rightward) $\longleftrightarrow$ **Facing West / Trailing** (points face right, diverge leftward).
- When a switch rotates, the attached reverse route reverses its corridor traversal direction.

### 4.2 Flip (`F`)
- Mirrors the turnout across the horizontal rail axis.
- **Diverge Down** (Right-Hand turnout, targets $\text{Level } L+1$) $\longleftrightarrow$ **Diverge Up** (Left-Hand turnout, targets $\text{Level } L-1$).
- The switch keeps its facing direction, but the reverse leg targets the opposite corridor.

### 4.3 Slot Insertion ("Insert into Grid")
- When you drop a new switch or block into a track:
  1. The Layout Manager inserts a new station column $S_{\text{new}}$ into the matrix.
  2. All parallel corridors expand their station columns together.
  3. Crossovers and parallel sidings maintain their vertical alignment without skew.

---

## 5. Dual-View Synchronization: Schematic vs. Model Board

The Schematic and the Model Board share the same Station Corridor Matrix, but serve different roles:

| Dimension | Engineering Schematic View | Virtual cTc Model Board View |
| :--- | :--- | :--- |
| **Purpose** | Planning, wiring, and configuration | Dispatching and live operation |
| **Space** | Expanded, scrollable canvas | Compact, fixed-aspect faceplate |
| **Electrical Detail** | Displays IRJs, pins, dropouts, sensor dots | Hides internal wiring and sensor nodes |
| **Track Display** | Dual rails with roadbed | Crisp single track line with jewel bulbs |
| **Signaling** | Wayside masts with cab line-of-sight | Small indication lamps (Clear / Stop) |
| **Lower Tier** | Property Inspector & Matrix Table | Mechanical Levers, Dogs, and Code Buttons |

---

## 6. Implementation Stages for Parallel Build

### Stage 1: The Station Corridor Matrix Engine (Rust Core)
- Implement `CorridorMatrix` with discrete corridors and station slots.
- Implement matrix operations: `insert_column`, `rotate_appliance`, `flip_appliance`.
- Unit tests verifying exact $0^\circ, 180^\circ, \pm 45^\circ$ constraints.

### Stage 2: Svelte Layout Renderer
- Map matrix slots to canvas coordinates automatically.
- Discard free-floating drag math in favor of slot-snapping.
- Render Schematic View and cTc Model Board View from the same matrix data.

### Stage 3: The Virtual cTc Faceplate
- Implement vertical lever columns directly aligned with the model board station slots.
- Wire switch levers, signal levers, and code buttons to Interface "A" snapshots.
