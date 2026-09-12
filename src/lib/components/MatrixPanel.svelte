<script lang="ts">
  import { studio } from '$lib/state.svelte';
  import type { SynthesizedRoute } from '$lib/types';

  function selectRoute(route: SynthesizedRoute) {
    if (studio.selectedRouteId === route.id) {
      studio.selectedRouteId = null;
    } else {
      studio.selectedRouteId = route.id;
    }
  }

  function formatAlignments(alignments: Record<string, string>): string {
    return Object.entries(alignments)
      .map(([sw, pos]) => `${sw}: ${pos === 'Normal' ? 'NORM' : 'REV'}`)
      .join(', ');
  }
</script>

<div class="bottom-panel" class:collapsed={!studio.isBottomPanelOpen}>
  <div class="panel-header">
    <div class="tab-buttons">
      <button
        class="tab-btn"
        class:active={studio.bottomTab === 'matrix'}
        onclick={() => (studio.bottomTab = 'matrix')}
      >
        Interlocking Control Table ({studio.project?.routes.length || 0})
      </button>
      <button
        class="tab-btn"
        class:active={studio.bottomTab === 'drc'}
        onclick={() => (studio.bottomTab = 'drc')}
      >
        Design Rule Checks ({studio.drcViolations.length})
      </button>
    </div>

    <button
      class="collapse-btn"
      onclick={() => (studio.isBottomPanelOpen = !studio.isBottomPanelOpen)}
    >
      {studio.isBottomPanelOpen ? '▼ Hide' : '▲ Show'}
    </button>
  </div>

  {#if studio.isBottomPanelOpen}
    <div class="panel-body">
      {#if studio.bottomTab === 'matrix'}
        {#if studio.project && studio.project.routes.length > 0}
          <div class="table-container">
            <table class="matrix-table">
              <thead>
                <tr>
                  <th>Route ID</th>
                  <th>Entrance</th>
                  <th>Dir</th>
                  <th>Switch Alignments</th>
                  <th>Cleared Circuits</th>
                  <th>Aspect Ceiling</th>
                  <th>Fleeting</th>
                  <th>Call-On</th>
                  <th>Conflicts</th>
                  <th>Concurrent</th>
                </tr>
              </thead>
              <tbody>
                {#each studio.project.routes as route}
                  <tr
                    class="route-row"
                    class:selected={studio.selectedRouteId === route.id}
                    onclick={() => selectRoute(route)}
                  >
                    <td class="monospace font-bold">{route.name}</td>
                    <td class="monospace text-blue">{route.entrance_signal_id}</td>
                    <td>{route.direction}</td>
                    <td class="monospace">{formatAlignments(route.switch_alignments)}</td>
                    <td class="monospace">{route.clears_circuits.join(', ')}</td>
                    <td>
                      <span class="aspect-badge aspect-{route.aspect_ceiling.toLowerCase()}">
                        {route.aspect_ceiling}
                      </span>
                    </td>
                    <td class="text-center">{route.fleeting_capable ? 'YES' : '—'}</td>
                    <td class="text-center">{route.call_on_capable ? 'YES' : '—'}</td>
                    <td class="text-amber">{route.conflicts_with.length}</td>
                    <td class="text-green">{route.concurrent_with.length}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {:else}
          <div class="empty-state">No synthesized routes yet. Click "Load Demo CP" or "Synthesize".</div>
        {/if}
      {:else}
        <!-- DRC Tab -->
        {#if studio.drcViolations.length > 0}
          <div class="drc-list">
            {#each studio.drcViolations as v}
              <div class="drc-item severity-{v.severity.toLowerCase()}">
                <span class="severity-tag">{v.severity.toUpperCase()}</span>
                <span class="drc-msg">{v.message}</span>
              </div>
            {/each}
          </div>
        {:else}
          <div class="empty-state text-green">All Design Rule Checks passed. Control Point is verified.</div>
        {/if}
      {/if}
    </div>
  {/if}
</div>

<style>
  .bottom-panel {
    background: #0f172a;
    border-top: 1px solid #1e293b;
    display: flex;
    flex-direction: column;
    user-select: none;
    max-height: 240px;
    transition: max-height 0.2s ease;
  }

  .bottom-panel.collapsed {
    max-height: 36px;
  }

  .panel-header {
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 14px;
    background: #090d16;
    border-bottom: 1px solid #1e293b;
  }

  .tab-buttons {
    display: flex;
    gap: 4px;
  }

  .tab-btn {
    background: transparent;
    border: none;
    color: #94a3b8;
    padding: 6px 12px;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    border-bottom: 2px solid transparent;
  }

  .tab-btn.active {
    color: #38bdf8;
    border-bottom-color: #38bdf8;
  }

  .collapse-btn {
    background: transparent;
    border: none;
    color: #64748b;
    font-size: 11px;
    cursor: pointer;
  }

  .panel-body {
    flex: 1;
    overflow-y: auto;
    padding: 8px 14px;
  }

  .table-container {
    overflow-x: auto;
  }

  .matrix-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 11px;
    color: #e2e8f0;
  }

  .matrix-table th {
    text-align: left;
    padding: 6px 8px;
    background: #1e293b;
    color: #94a3b8;
    font-weight: 600;
    border-bottom: 1px solid #334155;
  }

  .matrix-table td {
    padding: 6px 8px;
    border-bottom: 1px solid #1e293b;
  }

  .route-row {
    cursor: pointer;
    transition: background 0.1s ease;
  }

  .route-row:hover {
    background: #1e293b;
  }

  .route-row.selected {
    background: rgba(2, 132, 199, 0.25);
    border-left: 3px solid #38bdf8;
  }

  .aspect-badge {
    padding: 2px 6px;
    border-radius: 3px;
    font-size: 9px;
    font-weight: 700;
    text-transform: uppercase;
  }

  .aspect-clear {
    background: #15803d;
    color: #dcfce7;
  }

  .aspect-divergingclear {
    background: #0369a1;
    color: #e0f2fe;
  }

  .aspect-slowclear {
    background: #b45309;
    color: #fef3c7;
  }

  .monospace {
    font-family: monospace;
  }

  .text-blue {
    color: #38bdf8;
  }

  .text-green {
    color: #4ade80;
  }

  .text-amber {
    color: #fbbf24;
  }

  .text-center {
    text-align: center;
  }

  .empty-state {
    padding: 20px;
    text-align: center;
    color: #64748b;
    font-size: 12px;
  }

  .drc-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .drc-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 10px;
    border-radius: 4px;
    font-size: 11px;
    background: #1e293b;
  }

  .severity-tag {
    font-weight: 700;
    font-size: 9px;
    padding: 2px 6px;
    border-radius: 3px;
  }

  .severity-draft .severity-tag {
    background: #475569;
    color: #e2e8f0;
  }

  .severity-warning .severity-tag {
    background: #d97706;
    color: #fffbeb;
  }

  .severity-conflict .severity-tag {
    background: #dc2626;
    color: #fef2f2;
  }
</style>
