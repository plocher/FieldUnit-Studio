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

<header class="app-header">
  <!-- Top Menu Bar -->
  <div class="menu-bar">
    <div class="menu-left">
      <div class="logo-mark">FU</div>
      <div class="menu-item dropdown">
        <span class="menu-label">Edit</span>
        <div class="dropdown-menu">
          <button class="dropdown-btn" onclick={() => studio.undo()}>Undo (Cmd+Z)</button>
          <button class="dropdown-btn" onclick={() => studio.redo()}>Redo (Cmd+Shift+Z)</button>
          <hr class="menu-divider" />
          <button class="dropdown-btn" onclick={() => studio.deleteSelected()}>Delete Selected</button>
        </div>
      </div>
      <div class="menu-item dropdown">
        <span class="menu-label">Interlocking</span>
        <div class="dropdown-menu">
          <button class="dropdown-btn" onclick={() => studio.synthesizeRoutes()}>Synthesize Routes</button>
          <button class="dropdown-btn" onclick={() => studio.runDrc()}>Run DRC Validation</button>
          <hr class="menu-divider" />
          <button class="dropdown-btn" onclick={() => studio.loadDemo()}>Reload Demo CP</button>
        </div>
      </div>
      <div class="menu-item dropdown">
        <span class="menu-label">View</span>
        <div class="dropdown-menu">
          <button class="dropdown-btn" onclick={() => studio.autoArrange()}>Tidy Model Board (L)</button>
          <hr class="menu-divider" />
          <button class="dropdown-btn" onclick={() => studio.zoomIn()}>Zoom In (+)</button>
          <button class="dropdown-btn" onclick={() => studio.zoomOut()}>Zoom Out (-)</button>
          <button class="dropdown-btn" onclick={() => studio.resetZoom()}>Fit Normal (100%)</button>
          <hr class="menu-divider" />
          <button class="dropdown-btn" onclick={() => (studio.isLeftSidebarOpen = !studio.isLeftSidebarOpen)}>
            Toggle Palette
          </button>
          <button class="dropdown-btn" onclick={() => (studio.isRightSidebarOpen = !studio.isRightSidebarOpen)}>
            Toggle Inspector
          </button>
          <button class="dropdown-btn" onclick={() => (studio.isBottomPanelOpen = !studio.isBottomPanelOpen)}>
            Toggle Matrix Drawer
          </button>
        </div>
      </div>
    </div>

    <!-- Project Identifier -->
    <div class="menu-right">
      <span class="project-name">
        {studio.project ? studio.project.metadata.name : 'FieldUnit Studio'}
      </span>
      <span class="subdivision-tag">
        {studio.project ? studio.project.metadata.subdivision : 'No Project'}
      </span>
    </div>
  </div>

  <!-- Hanging File Folder Tabs -->
  <div class="folder-tabs-strip">
    {#each modes as mode}
      <button
        class="folder-tab"
        class:active={studio.workspaceMode === mode.id}
        onclick={() => (studio.workspaceMode = mode.id)}
      >
        <span class="tab-label">{mode.label}</span>
      </button>
    {/each}
  </div>
</header>

<style>
  .app-header {
    background-color: #0b0f17;
    border-bottom: 2px solid #1e293b;
    display: flex;
    flex-direction: column;
    user-select: none;
  }

  .menu-bar {
    height: 34px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 14px;
    border-bottom: 1px solid #161f2e;
    font-size: 11px;
  }

  .menu-left {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .logo-mark {
    width: 22px;
    height: 22px;
    border-radius: 4px;
    background: linear-gradient(135deg, #0284c7, #0369a1);
    color: #ffffff;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 800;
    font-size: 10px;
  }

  .menu-item {
    position: relative;
    cursor: pointer;
  }

  .menu-label {
    color: #94a3b8;
    font-weight: 500;
    padding: 4px 6px;
    border-radius: 3px;
  }

  .menu-label:hover {
    color: #f8fafc;
    background: #1e293b;
  }

  .dropdown-menu {
    display: none;
    position: absolute;
    top: 100%;
    left: 0;
    background: #182234;
    border: 1px solid #334155;
    border-radius: 4px;
    box-shadow: 0 8px 16px rgba(0, 0, 0, 0.4);
    min-width: 160px;
    z-index: 1000;
    padding: 4px 0;
  }

  .dropdown:hover .dropdown-menu {
    display: flex;
    flex-direction: column;
  }

  .dropdown-btn {
    background: transparent;
    border: none;
    text-align: left;
    padding: 6px 12px;
    font-size: 11px;
    color: #cbd5e1;
    cursor: pointer;
  }

  .dropdown-btn:hover {
    background: #0284c7;
    color: #ffffff;
  }

  .menu-divider {
    border: none;
    border-top: 1px solid #334155;
    margin: 4px 0;
  }

  .menu-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .project-name {
    color: #f1f5f9;
    font-weight: 600;
  }

  .subdivision-tag {
    color: #64748b;
    font-family: monospace;
  }

  /* Authentic Hanging File Folder Tabs */
  .folder-tabs-strip {
    display: flex;
    padding: 0 16px;
    background: #0b0f17;
    gap: 4px;
    margin-top: 2px;
  }

  .folder-tab {
    position: relative;
    background: #151e2c;
    border: 1px solid #243044;
    border-bottom: none;
    color: #94a3b8;
    padding: 7px 18px 6px;
    font-size: 11px;
    font-weight: 600;
    border-top-left-radius: 6px;
    border-top-right-radius: 6px;
    cursor: pointer;
    transition: all 0.12s ease;
  }

  .folder-tab:hover {
    color: #f1f5f9;
    background: #1c2738;
  }

  .folder-tab.active {
    background: #1e293b;
    color: #ffffff;
    border-color: #38bdf8;
    border-bottom: 2px solid #1e293b;
    margin-bottom: -2px;
    z-index: 10;
  }

  .folder-tab.active::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 3px;
    background: #38bdf8;
    border-top-left-radius: 4px;
    border-top-right-radius: 4px;
  }

  .tab-label {
    letter-spacing: 0.3px;
  }
</style>
