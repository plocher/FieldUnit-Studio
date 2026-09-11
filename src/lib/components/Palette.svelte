<script lang="ts">
  import { studio } from '$lib/state.svelte';

  const rawComponents = [
    { id: 'turnout', name: 'Turnout (Switch)', icon: '⑂', desc: 'Odd AAR switch appliance' },
    { id: 'crossover', name: 'Universal Crossover', icon: '⤧', desc: 'Paired switch machines' },
    { id: 'track', name: 'Track Net', icon: '—', desc: 'Auto-routed track line' },
    { id: 'irj', name: 'Insulated Joint (IRJ)', icon: '][', desc: 'Electrical block gap' },
    { id: 'sensor', name: 'Optical Sensor', icon: '(d)', desc: 'Frog fouling coverage' },
    { id: 'signal', name: 'Signal Mast', icon: '⚑', desc: 'Wayside governing mast' },
    { id: 'boundary', name: 'CP Boundary', icon: '⮥', desc: 'Interlocking limit flag' },
    { id: 'bumper', name: 'Track Bumper', icon: '⫿', desc: 'Stub buffer stop' },
  ];

  const recipes = [
    { id: 'end-of-siding', name: 'End-of-Siding CP', desc: '1 Switch, 3 IRJs, 3 Masts, Island 1T1' },
    { id: 'universal-xover', name: 'Universal Crossover', desc: '2 Switches, 4 IRJs, 4 Masts' },
  ];
</script>

<aside class="palette-sidebar">
  <div class="sidebar-header">
    <div class="section-title">Component Palette</div>
    <div class="subtitle">Drag to canvas</div>
  </div>

  <div class="palette-group">
    <div class="group-label">RAW APPLIANCES</div>
    <div class="item-grid">
      {#each rawComponents as comp}
        <div class="palette-item" title={comp.desc}>
          <div class="item-icon">{comp.icon}</div>
          <div class="item-label">{comp.name}</div>
        </div>
      {/each}
    </div>
  </div>

  <div class="palette-group">
    <div class="group-label">TURNKEY RECIPES</div>
    <div class="recipe-list">
      {#each recipes as recipe}
        <div class="recipe-card">
          <div class="recipe-title">{recipe.name}</div>
          <div class="recipe-desc">{recipe.desc}</div>
        </div>
      {/each}
    </div>
  </div>

  <div class="palette-group shortcuts-group">
    <div class="group-label">KEYBOARD SHORTCUTS</div>
    <div class="shortcut-row"><span class="key">A</span> Add Component</div>
    <div class="shortcut-row"><span class="key">W</span> Wire Track Net</div>
    <div class="shortcut-row"><span class="key">R</span> Rotate / Flip</div>
    <div class="shortcut-row"><span class="key">M</span> Move (Rubberband)</div>
    <div class="shortcut-row"><span class="key">T/E/S</span> Toggle Layers</div>
  </div>
</aside>

<style>
  .palette-sidebar {
    width: 220px;
    background-color: #0f172a;
    border-right: 1px solid #1e293b;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    user-select: none;
    color: #cbd5e1;
  }

  .sidebar-header {
    padding: 12px 14px;
    border-bottom: 1px solid #1e293b;
  }

  .section-title {
    font-size: 12px;
    font-weight: 700;
    color: #f8fafc;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .subtitle {
    font-size: 10px;
    color: #64748b;
    margin-top: 2px;
  }

  .palette-group {
    padding: 12px 14px;
    border-bottom: 1px solid #1e293b;
  }

  .group-label {
    font-size: 10px;
    font-weight: 700;
    color: #64748b;
    margin-bottom: 8px;
    letter-spacing: 0.5px;
  }

  .item-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
  }

  .palette-item {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 4px;
    padding: 8px 6px;
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    cursor: grab;
    transition: all 0.15s ease;
  }

  .palette-item:hover {
    background: #334155;
    border-color: #0284c7;
  }

  .item-icon {
    font-size: 16px;
    color: #38bdf8;
    margin-bottom: 4px;
  }

  .item-label {
    font-size: 10px;
    color: #e2e8f0;
    line-height: 1.2;
  }

  .recipe-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .recipe-card {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 4px;
    padding: 8px 10px;
    cursor: pointer;
  }

  .recipe-card:hover {
    border-color: #10b981;
    background: #1e293b;
  }

  .recipe-title {
    font-size: 11px;
    font-weight: 600;
    color: #f1f5f9;
  }

  .recipe-desc {
    font-size: 9px;
    color: #94a3b8;
    margin-top: 2px;
  }

  .shortcuts-group {
    margin-top: auto;
    font-size: 10px;
    color: #94a3b8;
    background: #090d16;
  }

  .shortcut-row {
    margin-bottom: 4px;
  }

  .key {
    background: #1e293b;
    border: 1px solid #475569;
    border-radius: 3px;
    padding: 1px 4px;
    color: #38bdf8;
    font-weight: 600;
    font-family: monospace;
    font-size: 10px;
  }
</style>
