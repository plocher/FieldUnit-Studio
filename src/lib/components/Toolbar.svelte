<script lang="ts">
  import { studio } from '$lib/state.svelte';
  import type { WorkspaceMode } from '$lib/types';

  const modes: { id: WorkspaceMode; label: string }[] = [
    { id: 'schematic', label: '1. Schematic Designer' },
    { id: 'matrix', label: '2. Interlocking Matrix' },
    { id: 'ctc', label: '3. Virtual cTc Machine' },
    { id: 'code', label: '4. Code & Sheets' },
  ];
</script>

<header class="app-toolbar">
  <!-- Brand & Project Title -->
  <div class="toolbar-section brand-section">
    <div class="logo-mark">FU</div>
    <div>
      <div class="app-title">FieldUnit Studio</div>
      <div class="project-subtitle">
        {studio.project ? `${studio.project.metadata.name} (${studio.project.metadata.subdivision})` : 'No Project Loaded'}
      </div>
    </div>
  </div>

  <!-- Workspace Mode Switcher -->
  <div class="toolbar-section mode-switcher">
    {#each modes as mode}
      <button
        class="mode-btn"
        class:active={studio.workspaceMode === mode.id}
        onclick={() => (studio.workspaceMode = mode.id)}
      >
        {mode.label}
      </button>
    {/each}
  </div>

  <!-- Layer Toggles -->
  <div class="toolbar-section layer-toggles">
    <button
      class="tool-btn layer-btn"
      class:active={studio.layers.track}
      onclick={() => studio.toggleLayer('track')}
      title="Toggle Track Layer [T]"
    >
      T: Track
    </button>
    <button
      class="tool-btn layer-btn"
      class:active={studio.layers.electrical}
      onclick={() => studio.toggleLayer('electrical')}
      title="Toggle Electrical Layer [E]"
    >
      E: Elec
    </button>
    <button
      class="tool-btn layer-btn"
      class:active={studio.layers.signals}
      onclick={() => studio.toggleLayer('signals')}
      title="Toggle Signal Layer [S]"
    >
      S: Signals
    </button>
    <button
      class="tool-btn layer-btn"
      class:active={studio.layers.speeds}
      onclick={() => studio.toggleLayer('speeds')}
      title="Toggle Speed Layer [R]"
    >
      R: Speeds
    </button>
    <button
      class="tool-btn layer-btn"
      class:active={studio.layers.names}
      onclick={() => studio.toggleLayer('names')}
      title="Toggle Names Layer [N]"
    >
      N: Names
    </button>
  </div>

  <!-- Actions -->
  <div class="toolbar-section actions-section">
    <button class="action-btn demo-btn" onclick={() => studio.loadDemo()}>
      Load Demo CP
    </button>
    <button class="action-btn" onclick={() => studio.synthesizeRoutes()}>
      Synthesize
    </button>
    <button class="action-btn" onclick={() => studio.runDrc()}>
      Run DRC
    </button>
    <div class="zoom-controls">
      <button class="icon-btn" onclick={() => studio.zoomOut()} title="Zoom Out">-</button>
      <button class="icon-btn" onclick={() => studio.resetZoom()} title="Fit View">100%</button>
      <button class="icon-btn" onclick={() => studio.zoomIn()} title="Zoom In">+</button>
    </div>
  </div>
</header>

<style>
  .app-toolbar {
    height: 52px;
    background-color: #0f172a;
    border-bottom: 1px solid #1e293b;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 16px;
    color: #e2e8f0;
    user-select: none;
  }

  .toolbar-section {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .brand-section {
    gap: 12px;
  }

  .logo-mark {
    width: 30px;
    height: 30px;
    border-radius: 6px;
    background: linear-gradient(135deg, #0284c7, #0369a1);
    color: #ffffff;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 800;
    font-size: 13px;
    letter-spacing: -0.5px;
  }

  .app-title {
    font-size: 13px;
    font-weight: 700;
    color: #f8fafc;
  }

  .project-subtitle {
    font-size: 11px;
    color: #64748b;
  }

  .mode-switcher {
    background: #1e293b;
    padding: 3px;
    border-radius: 6px;
  }

  .mode-btn {
    background: transparent;
    border: none;
    color: #94a3b8;
    padding: 5px 12px;
    font-size: 12px;
    font-weight: 500;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .mode-btn.active {
    background: #0284c7;
    color: #ffffff;
    font-weight: 600;
  }

  .tool-btn {
    background: #1e293b;
    border: 1px solid #334155;
    color: #94a3b8;
    padding: 4px 8px;
    font-size: 11px;
    font-weight: 600;
    border-radius: 4px;
    cursor: pointer;
  }

  .layer-btn.active {
    background: #0369a1;
    border-color: #38bdf8;
    color: #ffffff;
  }

  .action-btn {
    background: #1e293b;
    border: 1px solid #334155;
    color: #e2e8f0;
    padding: 5px 10px;
    font-size: 11px;
    font-weight: 600;
    border-radius: 4px;
    cursor: pointer;
  }

  .action-btn:hover {
    background: #334155;
  }

  .demo-btn {
    background: #10b981;
    border-color: #059669;
    color: #ffffff;
  }

  .demo-btn:hover {
    background: #059669;
  }

  .zoom-controls {
    display: flex;
    background: #1e293b;
    border-radius: 4px;
    overflow: hidden;
  }

  .icon-btn {
    background: transparent;
    border: none;
    color: #cbd5e1;
    padding: 4px 8px;
    font-size: 11px;
    cursor: pointer;
  }

  .icon-btn:hover {
    background: #334155;
  }
</style>
