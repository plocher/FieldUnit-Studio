# FieldUnit Studio: Model Board and cTc Architecture Specification

## 1. System Overview and Actor Separation

FieldUnit Studio separates the system into two operating environments: **Design Mode** for the plant engineer, and **Dispatch Mode** for the train dispatcher.

```
┌────────────────────────────────────────────────────────────────────────┐
│                      Design Mode (The Modeler)                         │
│  - Palette of turnkey recipes and compound appliances                  │
│  - Schematic editor enforcing horizontal lanes and 45° angles          │
│  - Background safety evaluator (continuous DRC & route discovery)       │
│  - Station column allocation and Milepost / Sequential numbering       │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
                                    │ Validate & Compile
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│                     Dispatch Mode (The Operator)                       │
│  - Condensed model board display with jewel occupancy lamps            │
│  - Physical lever deck aligned vertically under model board stations   │
│  - Local correspondence calculation (Demanded Intent vs. Field Truth)  │
│  - Interface "A" asynchronous CodeLine snapshot communications         │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Model Board Visual & Spatial Constraints

The model board is a stylized, simplified schematic. It enforces strict geometric constraints:

1. **Line Direction Limits**:
   - Track paths must follow only three angles: horizontal ($0^\circ / 180^\circ$), vertical ($90^\circ / 270^\circ$), or diagonal ($45^\circ / 135^\circ$).
   - Curved lines or non-standard angles are forbidden.

2. **Horizontal Track Corridors**:
   - Rails occupy discrete horizontal lanes (Corridor 0: Main Line, Corridor 1: Siding, Corridor 2: Spur).
   - Vertical distance between parallel corridors is fixed.

3. **Vertical Column Affinity**:
   - Every switch point, crossover, and signal cluster maintains vertical spatial affinity with its assigned control column on the lever deck below.

---

## 3. Lever Deck Physical & Mechanical Constraints

The lower half of the cTc machine represents the physical control plate:

1. **Station Column Assembly**:
   - The panel is an array of standardized vertical columns.
   - Each column contains:
     - Top row: Switch lever (odd number). Two-position rotary or toggle lever with $60^\circ$ detent throw (`N` up, `R` down).
     - Middle row: Signal lever (even number). Three-position lever with $45^\circ$ detent throw (`L` left, `STOP` center, `R` right).
     - Bottom row: Station Code Button (silver or brass machined button).
     - Optional aux toggles: Fleeting (`FS`), Call-On (`CO`), Maintainer Call (`MC`).

2. **Progressive Column Population**:
   - Placing a switch allocates a station column and populates the switch lever.
   - The signal lever slot remains a blank plate until the user assigns a governing signal.
   - Code buttons and auxiliary plates activate when associated appliances exist.

3. **Out-of-Correspondence (OOC) Indication**:
   - Moving a lever immediately drops correspondence on the console before the code button is punched.
   - GRS style: The barrel of the lever illuminates.
   - US&S style: Amber direction jewel lamps above the lever flash.
   - When points complete transit and lock in the commanded position, the steady indication lamp lights.

4. **Approach Time Locking (`2TEK`)**:
   - When a cleared signal is restored to Stop in front of an approaching train, the active direction lamp flashes for the countdown duration (e.g. 180 seconds).
   - Moving levers during countdown triggers transit alarms; points remain frozen.

5. **Mechanical Blocking Dogs**:
   - Right-click any lever to clamp a physical collar:
     - **Red Dog**: Track out of service or maintenance authority.
     - **Blue Dog**: Blue signal protection for carmen working.
   - Clamped levers cannot move.

---

## 4. Compound Appliances and Turnkey Recipes

Modelers rarely place raw components in isolation. Studio provides composite building blocks and complete recipes:

### 4.1 Compound Appliances
- **Switch Assembly**:
  - Dropping a switch places the switch points, frog, normal track stub, reverse track stub, dedicated island track circuit, and bounding insulated rail joints.
  - The reverse branch includes an optical fouling sensor at the frog clearance point.
- **Wayside Mast Assembly**:
  - Placed directly at an insulated rail joint.
  - Automatically orients heads toward approaching trains based on mounting side.
- **Detection Block**:
  - Drops a track section bounded by insulated rail joints and an assigned circuit identifier.

### 4.2 Turnkey Recipes
- **Passing Siding Control Point**:
  - Drops two switches, four track blocks (Island, Siding, Main, Advance), six signal masts, and eight insulated rail joints in proper alignment.
- **Universal Crossover**:
  - Drops two paired switches, four insulated rail joints, and governing signals across two parallel corridors.
- **Stub Terminal**:
  - Drops a switch, buffer stop bumper, track block, and governing dwarf signal.

### 4.3 Extensibility
- Custom recipes and compound assemblies are defined in standard JSON files.
- Users can save selected track clusters from the canvas as reusable custom recipes.

---

## 5. Naming Schemes and Layout Adaptations

Studio supports both North American prototype naming conventions:

1. **Sequential Lever Numbering**:
   - Switches receive odd numbers ($1, 3, 5\dots$).
   - Signals receive even numbers ($2, 4, 6\dots$).
   - Inserting a switch between 1 and 3 shifts subsequent columns rightward and re-indexes them.

2. **Milepost Numbering**:
   - Switches use milepost coordinates (e.g. Switch `801` at MP 80.1, Switch `833` at MP 83.3).
   - Signals use milepost with directional suffixes (e.g. `802R`, `802L`).
   - Inserting a switch between 801 and 833 opens a new station column (e.g. `815`) without changing existing switch numbers.

---

## 6. Heuristic Inference Rules

The background Evaluator automates configuration using topological rules:
- Siding stubs ending in bumpers $\implies$ Turnout speed defaults to **Slow** (15 mph).
- Parallel crossover connections $\implies$ Turnout speed defaults to **Normal** or **Limited**.
- Siding passing loops $\implies$ Turnout speed defaults to **Medium** (30 mph).
- Switch within an interlocking $\implies$ Evaluator requires dedicated island block and optical sensor.
- Modeler override: The Property Inspector always allows manual override of inferred speeds, pinouts, and timers.
