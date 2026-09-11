<script lang="ts">
  import { studio } from '$lib/state.svelte';

  const rawComponents = [
    { id: 'turnout', name: 'Turnout', type: 'turnout', desc: 'Odd AAR switch appliance' },
    { id: 'crossover', name: 'Crossover', type: 'crossover', desc: 'Paired switch machines' },
    { id: 'block', name: 'Block', type: 'block', desc: 'Track detection block with IRJ' },
    { id: 'irj', name: 'IRJ Joint', type: 'irj', desc: 'Insulated rail gap' },
    { id: 'sensor', name: 'Optical Sensor', type: 'sensor', desc: 'Frog fouling coverage' },
    { id: 'signal', name: 'Signal Mast', type: 'signal', desc: 'Wayside governing mast' },
    { id: 'boundary', name: 'CP Boundary', type: 'boundary', desc: 'Interlocking limit marker' },
    { id: 'bumper', name: 'Bumper', type: 'bumper', desc: 'Buffer stop' },
  ];

  const recipes = [
    { id: 'end-of-siding', name: 'End-of-Siding CP', desc: '1 Switch, 3 IRJs, 3 Masts, Island 1T1' },
    { id: 'universal-xover', name: 'Universal Crossover', desc: '2 Switches, 4 IRJs, 4 Masts' },
  ];

  const layerItems: { key: keyof typeof studio.layers; name: string; keyHint: string; color: string }[] = [
    { key: 'track', name: 'Track & Rails', keyHint: 'T', color: '#94a3b8' },
    { key: 'electrical', name: 'Electrical & IRJ', keyHint: 'E', color: '#ef4444' },
    { key: 'signals', name: 'Signal Masts', keyHint: 'S', color: '#22c55e' },
    { key: 'speeds', name: 'Turnout Speeds', keyHint: 'R', color: '#f59e0b' },
    { key: 'names', name: 'Nomenclature', keyHint: 'N', color: '#38bdf8' },
  ];
</script>

<aside class="palette-sidebar">
  <div class="sidebar-header">
    <div class="section-title">EDA PALETTE</div>
    <div class="subtitle">Drag & Drop Appliances</div>
  </div>

  <!-- KiCad-style Layer Manager -->
  <div class="palette-group">
    <div class="group-label">LAYERS MANAGER</div>
    <div class="layer-list">
      {#each layerItems as item}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="layer-item"
          class:active={studio.layers[item.key]}
          onclick={() => studio.toggleLayer(item.key)}
        >
          <span class="layer-swatch" style="background-color: {item.color};"></span>
          <span class="layer-name">{item.name}</span>
          <span class="key-hint">{item.keyHint}</span>
        </div>
      {/each}
    </div>
  </div>

  <div class="palette-group">
    <div class="group-label">RAW APPLIANCES (CLICK TO ARM)</div>
    <div class="item-grid">
      {#each rawComponents as comp}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="palette-item"
          class:armed={studio.activeTool === comp.id}
          title={comp.desc}
          onclick={() => {
            studio.activeTool = studio.activeTool === comp.id ? null : comp.id;
          }}
        >
          <div class="item-icon-box">
            {#if comp.type === 'turnout'}
              <svg width="32" height="22" viewBox="0 0 32 22">
                <line x1="2" y1="11" x2="30" y2="11" stroke="#94a3b8" stroke-width="2.5" />
                <line x1="12" y1="11" x2="26" y2="3" stroke="#94a3b8" stroke-width="2.5" />
                <circle cx="12" cy="11" r="3" fill="#f59e0b" stroke="#ffffff" stroke-width="1" />
              </svg>
            {:else if comp.type === 'signal'}
              <svg width="32" height="22" viewBox="0 0 32 22">
                <line x1="4" y1="3" x2="4" y2="19" stroke="#cbd5e1" stroke-width="1.8" />
                <line x1="4" y1="11" x2="14" y2="11" stroke="#cbd5e1" stroke-width="1.8" />
                <rect x="14" y="5" width="16" height="12" rx="3" fill="#0f172a" stroke="#e2e8f0" stroke-width="1" />
                <circle cx="18" cy="11" r="2.5" fill="#ef4444" />
                <circle cx="25" cy="11" r="2.5" fill="#22c55e" />
              </svg>
            {:else if comp.type === 'irj'}
              <svg width="32" height="22" viewBox="0 0 32 22">
                <line x1="2" y1="11" x2="30" y2="11" stroke="#94a3b8" stroke-width="2.5" />
                <line x1="13" y1="4" x2="13" y2="18" stroke="#ef4444" stroke-width="2" />
                <line x1="19" y1="4" x2="19" y2="18" stroke="#ef4444" stroke-width="2" />
              </svg>
            {:else if comp.type === 'boundary'}
              <svg width="32" height="22" viewBox="0 0 32 22">
                <rect x="9" y="3" width="14" height="16" rx="2" fill="#3b82f6" fill-opacity="0.3" stroke="#60a5fa" stroke-width="1.2" />
                <line x1="16" y1="2" x2="16" y2="20" stroke="#60a5fa" stroke-width="1.8" />
              </svg>
            {:else if comp.type === 'bumper'}
              <svg width="32" height="22" viewBox="0 0 32 22">
                <line x1="2" y1="11" x2="22" y2="11" stroke="#94a3b8" stroke-width="2.5" />
                <rect x="22" y="4" width="4" height="14" fill="#ef4444" stroke="#ffffff" stroke-width="1" />
              </svg>
            {:else if comp.type === 'crossover'}
              <svg width="32" height="22" viewBox="0 0 32 22">
                <line x1="2" y1="5" x2="30" y2="5" stroke="#94a3b8" stroke-width="2" />
                <line x1="2" y1="17" x2="30" y2="17" stroke="#94a3b8" stroke-width="2" />
                <line x1="8" y1="17" x2="24" y2="5" stroke="#94a3b8" stroke-width="2" />
              </svg>
            {:else if comp.type === 'sensor'}
              <svg width="32" height="22" viewBox="0 0 32 22">
                <circle cx="16" cy="11" r="5" fill="#f59e0b" fill-opacity="0.3" stroke="#f59e0b" stroke-width="1.5" />
                <text x="16" y="14" text-anchor="middle" fill="#fbbf24" font-size="8" font-family="monospace">d</text>
              </svg>
            {:else}
              <!-- Detection Block Icon -->
              <svg width="32" height="22" viewBox="0 0 32 22">
                <line x1="2" y1="11" x2="30" y2="11" stroke="#38bdf8" stroke-width="2.5" />
                <line x1="7" y1="5" x2="7" y2="17" stroke="#ef4444" stroke-width="1.8" />
                <line x1="11" y1="5" x2="11" y2="17" stroke="#ef4444" stroke-width="1.8" />
              </svg>
            {/if}
          </div>
          <div class="item-label">
            {comp.name}
            {#if studio.activeTool === comp.id}
              <span class="armed-tag">ARMED</span>
            {/if}
          </div>
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
    <div class="group-label">QUICK SHORTCUTS</div>
    <div class="shortcut-row"><span class="key">A</span> Add Component</div>
    <div class="shortcut-row"><span class="key">W</span> Wire Track Net</div>
    <div class="shortcut-row"><span class="key">R</span> Rotate / Flip</div>
    <div class="shortcut-row"><span class="key">M</span> Move (Rubberband)</div>
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
    padding: 10px 14px;
    border-bottom: 1px solid #1e293b;
  }

  .section-title {
    font-size: 11px;
    font-weight: 700;
    color: #f8fafc;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .subtitle {
    font-size: 10px;
    color: #64748b;
    margin-top: 1px;
  }

  .palette-group {
    padding: 10px 14px;
    border-bottom: 1px solid #1e293b;
  }

  .group-label {
    font-size: 9px;
    font-weight: 700;
    color: #64748b;
    margin-bottom: 6px;
    letter-spacing: 0.5px;
  }

  /* Layers Manager */
  .layer-list {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .layer-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 8px;
    border-radius: 4px;
    font-size: 11px;
    cursor: pointer;
    background: #151d2a;
    border: 1px solid #233146;
    opacity: 0.5;
    transition: all 0.1s ease;
  }

  .layer-item.active {
    opacity: 1;
    background: #1e293b;
    border-color: #38bdf8;
  }

  .layer-swatch {
    width: 8px;
    height: 8px;
    border-radius: 2px;
  }

  .layer-name {
    flex: 1;
    color: #e2e8f0;
  }

  .key-hint {
    font-size: 9px;
    font-family: monospace;
    color: #64748b;
    background: #0f172a;
    padding: 1px 4px;
    border-radius: 2px;
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

  .palette-item.armed {
    background: #0369a1;
    border-color: #38bdf8;
  }

  .armed-tag {
    display: block;
    font-size: 8px;
    font-weight: 800;
    color: #38bdf8;
    margin-top: 2px;
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
