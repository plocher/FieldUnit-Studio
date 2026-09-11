# FieldUnit Studio

**Visual Interlocking Designer, Virtual cTc Simulator, and Code Generator**

FieldUnit Studio is a responsive, cross-platform desktop application (macOS, Linux, Windows) designed to plan, synthesize, simulate, and operate railroad control points and interlockings.

It pairs directly with the [FieldUnit C++ Arduino Library](https://github.com/plocher/FieldUnit).

---

## The Four Primary Workspaces

1. **Schematic Track Designer (EDA Metaphor)**:
   - Component-based placement (Switches, Crossovers, Signals, Bumpers, CP Boundaries).
   - Auto-routing track nets with orthogonal and 45-degree constraints.
   - Five-layer visibility system (Track, Electrical, Signals, Operations/Speeds, Nomenclature).
   - Semantic zoom (LOD) and CP focus/isolation modes for large layout subdivisions.
   - Topological heuristics (90% auto-tagging of speeds and clearance blocks).

2. **Automated Route Synthesizer & Interlocking Matrix**:
   - Directed graph path discovery from entrance signals to exit boundaries.
   - Conflict groups (opposing moves) and concurrent route discovery (parallel moves).
   - Aspect ceiling calculation based on operational speed classes (Normal, Limited, Medium, Slow).

3. **Virtual cTc Machine (Simulation & Live Layout Dual-Use)**:
   - Authentic US&S and GRS console styling (60° switch detents, 45° signal detents, code buttons, mechanical blocking dogs).
   - Out-of-correspondence (OOC) and approach time locking (`2TEK`) flashing indications.
   - Stepper relay acoustic clatter and OS bell chimes.
   - Drag-and-drop Ghost Train simulation layer with engine audio.
   - **Live Layout Mode**: Operates directly over Interface "A" (Serial / RS-485 / TCP / MQTT) or Interface "B" with zero host-side ladder logic.

4. **Code, Firmware, and Shop Documentation Generator**:
   - Declarative C++ `configurePlant()` code generation.
   - Dynamic `plant.json` export for runtime flash/LittleFS loading.
   - Direct firmware build and flash via `arduino-cli`.
   - Dual-format shop wiring sheets (Markdown and printable 8.5" x 11" PDF with solder check-off boxes).
   - Complete AAR / AREMA "Packet-Lite" engineering packages.

---

## Technical Stack

- **Shell**: [Tauri v2](https://v2.tauri.app/) (Rust core)
- **Frontend**: [Svelte 5](https://svelte.dev/) + TypeScript + Vite
- **Hardware I/O**: Native asynchronous serial (`serialport`), TCP/UDP, and MQTT (`rumqttc`)

---

## Development Setup

```bash
# Install Node dependencies
npm install

# Run in desktop development mode
npm run tauri dev

# Build production bundle
npm run tauri build
```

Full architecture and design details are available in [`docs/FIELDUNIT_STUDIO_DESIGN_SPEC.md`](docs/FIELDUNIT_STUDIO_DESIGN_SPEC.md).
