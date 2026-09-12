<script lang="ts">
  import { studio } from '$lib/state.svelte';
  import type { SpeedClass } from '$lib/types';

  const speeds: SpeedClass[] = ['Slow', 'Medium', 'Limited', 'Normal'];
</script>

<aside class="inspector-sidebar">
  <div class="sidebar-header">
    <div class="section-title">Property Inspector</div>
    <div class="subtitle">
      {studio.selectedNodeId ? `Node: ${studio.selectedNodeId}` : 'Plant Overview'}
    </div>
  </div>

  {#if studio.project}
    {@const cp = studio.project.control_points[0]}
    <div class="inspector-content">
      {#if studio.selectedCircuitId}
        {@const circuit = cp?.track_circuits.find((c) => c.id === studio.selectedCircuitId)}
        <div class="prop-group">
          <div class="group-title">DETECTION BLOCK (CIRCUIT)</div>
          <div class="prop-row">
            <span class="prop-label">Block ID:</span>
            <span class="prop-val monospace">{studio.selectedCircuitId}</span>
          </div>
          <div class="prop-row">
            <span class="prop-label">Role:</span>
            <span class="prop-val">{circuit?.is_island ? 'Island (OS Section)' : 'Block Circuit'}</span>
          </div>
          <div class="prop-row">
            <span class="prop-label">Dropout Delay:</span>
            <span class="prop-val">{circuit?.dropout_delay_ms ?? (circuit?.is_island ? 2000 : 100)} ms</span>
          </div>
          <div class="prop-row">
            <span class="prop-label">Current Pin:</span>
            <input class="prop-input" type="number" placeholder="DCCOD pin" />
          </div>
          <div class="prop-row">
            <span class="prop-label">Optical Pin:</span>
            <input class="prop-input" type="number" placeholder="Frog photodiode" />
          </div>
        </div>
      {:else if studio.selectedNodeId && studio.project.graph.nodes[studio.selectedNodeId]}
        {@const node = studio.project.graph.nodes[studio.selectedNodeId]}
        <div class="prop-group">
          <div class="group-title">SELECTED NODE</div>
          <div class="prop-row">
            <span class="prop-label">Node ID:</span>
            <span class="prop-val monospace">{node.id}</span>
          </div>
          <div class="prop-row">
            <span class="prop-label">Position X:</span>
            <span class="prop-val">{node.x}</span>
          </div>
          <div class="prop-row">
            <span class="prop-label">Position Y:</span>
            <span class="prop-val">{node.y}</span>
          </div>
          <button class="delete-btn" onclick={() => studio.deleteSelected()}>
            Delete Selected ({studio.selectedNodeIds.length || 1})
          </button>
        </div>

        {#if 'SwitchPoints' in node.kind}
          {@const swId = node.kind.SwitchPoints.switch_id}
          {@const sw = cp?.switches.find((s) => s.id === swId)}
          <div class="prop-group">
            <div class="group-title">SWITCH APPLIANCE</div>
            <div class="prop-row">
              <span class="prop-label">AAR ID:</span>
              <span class="prop-val monospace">{sw?.id || swId}</span>
            </div>
            <div class="prop-row">
              <span class="prop-label">Orientation:</span>
              <span class="prop-val text-amber font-bold">
                {sw?.orientation === 'FacingEastDivergeUp' ? 'Facing East (Up)' :
                 sw?.orientation === 'FacingWestDivergeDown' ? 'Facing West (Down)' :
                 sw?.orientation === 'FacingWestDivergeUp' ? 'Facing West (Up)' :
                 'Facing East (Down)'}
              </span>
            </div>
            <div class="rotate-btn-row">
              <button class="rotate-btn" onclick={() => studio.rotateSelectedSwitch()} title="Rotate facing direction East <-> West (R)">
                ⤹ Rotate (R)
              </button>
              <button class="rotate-btn flip-btn" onclick={() => studio.flipSelectedSwitch()} title="Flip diverge side Up <-> Down (F)">
                ⇅ Flip (F)
              </button>
            </div>
            <div class="prop-row" style="margin-top: 8px;">
              <span class="prop-label">Speed:</span>
              <select class="prop-select" bind:value={sw!.speed}>
                {#each speeds as s}
                  <option value={s}>{s}</option>
                {/each}
              </select>
            </div>
            <div class="prop-row">
              <span class="prop-label">Motor Pin:</span>
              <input class="prop-input" type="number" placeholder="e.g. 12" />
            </div>
            <div class="prop-row">
              <span class="prop-label">Sense Normal:</span>
              <input class="prop-input" type="number" placeholder="e.g. 14" />
            </div>
            <div class="prop-row">
              <span class="prop-label">Sense Reverse:</span>
              <input class="prop-input" type="number" placeholder="e.g. 15" />
            </div>
          </div>
        {:else if 'Irj' in node.kind}
          <div class="prop-group">
            <div class="group-title">INSULATED RAIL JOINT</div>
            <div class="prop-row">
              <span class="prop-label">Left Circuit:</span>
              <span class="prop-val monospace">{node.kind.Irj.circuit_left}</span>
            </div>
            <div class="prop-row">
              <span class="prop-label">Right Circuit:</span>
              <span class="prop-val monospace">{node.kind.Irj.circuit_right}</span>
            </div>
          </div>
        {/if}
      {:else}
        <!-- Plant Overview -->
        <div class="prop-group">
          <div class="group-title">CONTROL POINT</div>
          <div class="prop-row">
            <span class="prop-label">Name:</span>
            <span class="prop-val">{cp?.name || 'CP'}</span>
          </div>
          <div class="prop-row">
            <span class="prop-label">Subdivision:</span>
            <span class="prop-val">{cp?.subdivision || 'Sub'}</span>
          </div>
          <div class="prop-row">
            <span class="prop-label">Rulebook:</span>
            <span class="prop-val">{studio.project.metadata.rulebook}</span>
          </div>
        </div>

        <div class="prop-group">
          <div class="group-title">APPLIANCE COUNTS</div>
          <div class="prop-row">
            <span class="prop-label">Switches:</span>
            <span class="prop-val">{cp?.switches.length || 0}</span>
          </div>
          <div class="prop-row">
            <span class="prop-label">Track Circuits:</span>
            <span class="prop-val">{cp?.track_circuits.length || 0}</span>
          </div>
          <div class="prop-row">
            <span class="prop-label">Signal Masts:</span>
            <span class="prop-val">{cp?.signal_masts.length || 0}</span>
          </div>
          <div class="prop-row">
            <span class="prop-label">Boundaries:</span>
            <span class="prop-val">{cp?.boundaries.length || 0}</span>
          </div>
        </div>
      {/if}
    </div>
  {/if}
</aside>

<style>
  .inspector-sidebar {
    width: 250px;
    background-color: #0f172a;
    border-left: 1px solid #1e293b;
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

  .inspector-content {
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .prop-group {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 4px;
    padding: 10px 12px;
  }

  .group-title {
    font-size: 10px;
    font-weight: 700;
    color: #38bdf8;
    margin-bottom: 8px;
    letter-spacing: 0.5px;
  }

  .prop-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 11px;
    margin-bottom: 6px;
  }

  .prop-label {
    color: #94a3b8;
  }

  .prop-val {
    color: #f1f5f9;
    font-weight: 500;
  }

  .prop-val.monospace {
    font-family: monospace;
    color: #38bdf8;
  }

  .prop-select,
  .prop-input {
    background: #0f172a;
    border: 1px solid #475569;
    color: #f8fafc;
    border-radius: 3px;
    padding: 2px 6px;
    font-size: 11px;
    width: 90px;
  }

  .delete-btn {
    width: 100%;
    margin-top: 8px;
    background: rgba(220, 38, 38, 0.2);
    border: 1px solid #ef4444;
    color: #fca5a5;
    padding: 6px 10px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .delete-btn:hover {
    background: #dc2626;
    color: #ffffff;
  }

  .rotate-btn-row {
    display: flex;
    gap: 6px;
    margin-top: 4px;
  }

  .rotate-btn {
    flex: 1;
    background: #1e293b;
    border: 1px solid #0284c7;
    color: #38bdf8;
    padding: 6px 6px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .rotate-btn:hover {
    background: #0284c7;
    color: #ffffff;
  }

  .flip-btn {
    border-color: #f59e0b;
    color: #fbbf24;
  }

  .flip-btn:hover {
    background: #d97706;
    color: #ffffff;
  }

  .text-amber {
    color: #f59e0b;
  }

  .font-bold {
    font-weight: 700;
  }
</style>
