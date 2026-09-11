<script lang="ts">
  import { studio } from '$lib/state.svelte';
  import type { TrackNode, TrackEdge } from '$lib/types';

  let svgElement: SVGSVGElement | null = $state(null);
  let isPanning = $state(false);
  let panStartX = $state(0);
  let panStartY = $state(0);

  let draggingNodeId = $state<string | null>(null);
  let dragOffset = $state({ x: 0, y: 0 });

  function handleWheel(event: WheelEvent) {
    event.preventDefault();
    const factor = event.deltaY < 0 ? 1.08 : 0.92;
    studio.setZoom(studio.zoom * factor);
  }

  function handleMouseDown(event: MouseEvent) {
    if (event.button === 1 || (event.button === 0 && (event.target as HTMLElement).tagName === 'svg')) {
      isPanning = true;
      panStartX = event.clientX - studio.panX;
      panStartY = event.clientY - studio.panY;
    }
  }

  function handleMouseMove(event: MouseEvent) {
    if (isPanning) {
      studio.panX = event.clientX - panStartX;
      studio.panY = event.clientY - panStartY;
    } else if (draggingNodeId && studio.project) {
      const rect = svgElement?.getBoundingClientRect();
      if (!rect) return;
      const mouseX = (event.clientX - rect.left - studio.panX) / studio.zoom;
      const mouseY = (event.clientY - rect.top - studio.panY) / studio.zoom;
      studio.updateNodePosition(draggingNodeId, mouseX - dragOffset.x, mouseY - dragOffset.y);
    }
  }

  function handleMouseUp() {
    isPanning = false;
    draggingNodeId = null;
  }

  function startNodeDrag(event: MouseEvent, node: TrackNode) {
    event.stopPropagation();
    draggingNodeId = node.id;
    studio.selectedNodeId = node.id;
    const rect = svgElement?.getBoundingClientRect();
    if (!rect) return;
    const mouseX = (event.clientX - rect.left - studio.panX) / studio.zoom;
    const mouseY = (event.clientY - rect.top - studio.panY) / studio.zoom;
    dragOffset = { x: mouseX - node.x, y: mouseY - node.y };
  }

  function getEdgeCircuitId(edge: TrackEdge): string {
    if ('Tangent' in edge.kind) return edge.kind.Tangent.circuit_id;
    if ('SwitchNormal' in edge.kind) return edge.kind.SwitchNormal.circuit_id;
    if ('SwitchReverse' in edge.kind) return edge.kind.SwitchReverse.circuit_id;
    return '';
  }

  function isEdgeInActiveRoute(edge: TrackEdge): boolean {
    if (!studio.activeRoute) return false;
    const cId = getEdgeCircuitId(edge);
    return studio.activeRoute.clears_circuits.includes(cId);
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="canvas-container"
  onwheel={handleWheel}
  onmousedown={handleMouseDown}
  onmousemove={handleMouseMove}
  onmouseup={handleMouseUp}
>
  <svg bind:this={svgElement} class="schematic-svg">
    <defs>
      <!-- Dot grid pattern -->
      <pattern id="grid-dots" width="20" height="20" patternUnits="userSpaceOnUse">
        <circle cx="2" cy="2" r="1" fill="#333b47" />
      </pattern>
      <!-- Active route glow filter -->
      <filter id="route-glow" x="-20%" y="-20%" width="140%" height="140%">
        <feGaussianBlur stdDeviation="4" result="blur" />
        <feComposite in="SourceGraphic" in2="blur" operator="over" />
      </filter>
    </defs>

    <!-- Canvas Background -->
    <rect width="100%" height="100%" fill="#141820" />
    <g transform="translate({studio.panX}, {studio.panY}) scale({studio.zoom})">
      <rect x="-3000" y="-3000" width="6000" height="6000" fill="url(#grid-dots)" opacity="0.8" />

      {#if studio.project}
        <!-- Control Point Boundary Box -->
        <!-- Extends to slightly less than half of boundary IRJs (x: 222 to 578) -->
        <rect
          x="222"
          y="130"
          width="356"
          height="190"
          rx="8"
          fill="#10b981"
          fill-opacity="0.04"
          stroke="#10b981"
          stroke-opacity="0.35"
          stroke-width="1.5"
          stroke-dasharray="5 5"
        />
        {#if studio.layers.names}
          <text x="232" y="148" fill="#10b981" font-size="11" font-weight="700" letter-spacing="0.5">
            CP END OF SIDING [VERIFIED]
          </text>
        {/if}

        <!-- 1. Track Layer (Edges / Rails) -->
        {#if studio.layers.track}
          {#each studio.project.graph.edges as edge}
            {@const fromNode = studio.project.graph.nodes[edge.from]}
            {@const toNode = studio.project.graph.nodes[edge.to]}
            {#if fromNode && toNode}
              {@const isHighlighted = isEdgeInActiveRoute(edge)}
              {@const midX = (fromNode.x + toNode.x) / 2}
              {@const midY = (fromNode.y + toNode.y) / 2}
              {@const circuitId = getEdgeCircuitId(edge)}

              <!-- Roadbed Shadow -->
              <line
                x1={fromNode.x}
                y1={fromNode.y}
                x2={toNode.x}
                y2={toNode.y}
                stroke="#090d16"
                stroke-width="8"
                stroke-linecap="round"
              />
              <!-- Steel Rails -->
              <line
                x1={fromNode.x}
                y1={fromNode.y}
                x2={toNode.x}
                y2={toNode.y}
                stroke={isHighlighted ? '#38bdf8' : '#94a3b8'}
                stroke-width={isHighlighted ? 4 : 3}
                stroke-linecap="round"
                filter={isHighlighted ? 'url(#route-glow)' : 'none'}
              />

              <!-- Block Names directly overlaying track line -->
              {#if studio.layers.electrical && circuitId && edge.length_feet >= 100}
                <g transform="translate({midX}, {midY})">
                  <rect x="-16" y="-7" width="32" height="14" rx="2" fill="#0f172a" stroke="#334155" stroke-width="1" />
                  <text x="0" y="3.5" text-anchor="middle" fill="#38bdf8" font-size="9" font-family="monospace" font-weight="600">
                    {circuitId}
                  </text>
                </g>
              {/if}

              <!-- Speed Overlay on Diverging Branches -->
              {#if studio.layers.speeds && 'SwitchReverse' in edge.kind}
                <text
                  x={midX + 8}
                  y={midY - 8}
                  fill="#f59e0b"
                  font-size="10"
                  font-weight="bold"
                >
                  {edge.kind.SwitchReverse.speed.toUpperCase()} (30 MPH)
                </text>
              {/if}
            {/if}
          {/each}
        {/if}

        <!-- 2. Nodes (IRJ, Switch Points, Boundaries) -->
        {#each Object.values(studio.project.graph.nodes) as node}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <g
            class="node-group"
            transform="translate({node.x}, {node.y})"
            onmousedown={(e) => startNodeDrag(e, node)}
          >
            {#if 'Boundary' in node.kind}
              <!-- CP Boundary Limit Marker -->
              <rect x="-8" y="-14" width="16" height="28" rx="2" fill="#3b82f6" fill-opacity="0.2" stroke="#60a5fa" stroke-width="1.5" />
              <line x1="0" y1="-14" x2="0" y2="14" stroke="#60a5fa" stroke-width="2" />
              {#if studio.layers.names}
                <text x="0" y="24" text-anchor="middle" fill="#93c5fd" font-size="10" font-weight="600">
                  {node.kind.Boundary.boundary_id}
                </text>
              {/if}
            {:else if 'Irj' in node.kind}
              <!-- Insulated Rail Joint Symbol ][ -->
              {#if studio.layers.electrical}
                <line x1="-3" y1="-10" x2="-3" y2="10" stroke="#ef4444" stroke-width="2.5" />
                <line x1="3" y1="-10" x2="3" y2="10" stroke="#ef4444" stroke-width="2.5" />
              {/if}
            {:else if 'SwitchPoints' in node.kind}
              <!-- Switch Points Node -->
              <circle cx="0" cy="0" r="5" fill="#f59e0b" stroke="#ffffff" stroke-width="1.5" />
              {#if studio.layers.names}
                <text x="0" y="-12" text-anchor="middle" fill="#fbbf24" font-size="11" font-weight="bold">
                  SW1 (ODD)
                </text>
              {/if}
            {:else if 'Bumper' in node.kind}
              <rect x="-4" y="-8" width="8" height="16" fill="#ef4444" stroke="#ffffff" stroke-width="1" />
            {/if}

            {#if studio.selectedNodeId === node.id}
              <circle cx="0" cy="0" r="14" fill="none" stroke="#38bdf8" stroke-width="2" stroke-dasharray="3 3" />
            {/if}
          </g>
        {/each}

        <!-- 3. Signal Layer (Wayside Signaling with Engineer Cab Perspective) -->
        {#if studio.layers.signals}
          {#each studio.project.control_points as cp}
            {#each cp.signal_masts as mast}
              {@const irjNode = mast.irj_node_id ? studio.project.graph.nodes[mast.irj_node_id] : null}
              {#if irjNode}
                {@const isRight = mast.direction === 'Right'}
                <!--
                  Engineer sitting in right-hand seat:
                  - Southbound (traffic right): signal sits BELOW track.
                    Base | aligns with IRJ. Arm - extends LEFT towards approaching train. Heads oo face left: |-oo
                  - Northbound (traffic left): signal sits ABOVE track.
                    Base | aligns with IRJ. Arm - extends RIGHT towards approaching train. Heads oo face right: oo-|
                -->
                <g class="signal-mast-group">
                  {#if isRight}
                    <!-- 2Sab: Below rail, base at IRJ, arm extending left -->
                    <g transform="translate({irjNode.x}, {irjNode.y + 14})">
                      <!-- Mast Base | aligned to IRJ -->
                      <line x1="0" y1="0" x2="0" y2="16" stroke="#e2e8f0" stroke-width="3" stroke-linecap="round" />
                      <!-- Horizontal arm extending left -->
                      <line x1="0" y1="8" x2="-18" y2="8" stroke="#cbd5e1" stroke-width="2" />
                      <!-- Heads facing oncoming train (facing Left) -->
                      <circle cx="-22" cy="4" r="3.5" fill="#22c55e" stroke="#0f172a" stroke-width="1" />
                      <circle cx="-22" cy="12" r="3.5" fill="#ef4444" stroke="#0f172a" stroke-width="1" />
                      {#if studio.layers.names}
                        <text x="-28" y="11" text-anchor="end" fill="#f8fafc" font-size="11" font-weight="700">
                          {mast.name}
                        </text>
                      {/if}
                    </g>
                  {:else}
                    <!-- 2Nab / 2Nc: Above rail, base at IRJ, arm extending right -->
                    <g transform="translate({irjNode.x}, {irjNode.y - 14})">
                      <!-- Mast Base | aligned to IRJ -->
                      <line x1="0" y1="0" x2="0" y2="-16" stroke="#e2e8f0" stroke-width="3" stroke-linecap="round" />
                      <!-- Horizontal arm extending right -->
                      <line x1="0" y1="-8" x2="18" y2="-8" stroke="#cbd5e1" stroke-width="2" />
                      <!-- Heads facing oncoming train (facing Right) -->
                      {#if mast.mast_type === 'TwoHead'}
                        <circle cx="22" cy="-12" r="3.5" fill="#22c55e" stroke="#0f172a" stroke-width="1" />
                        <circle cx="22" cy="-4" r="3.5" fill="#ef4444" stroke="#0f172a" stroke-width="1" />
                      {:else}
                        <!-- Dwarf signal (single head c) -->
                        <circle cx="22" cy="-8" r="3.5" fill="#ef4444" stroke="#0f172a" stroke-width="1" />
                      {/if}
                      {#if studio.layers.names}
                        <text x="28" y="-5" text-anchor="start" fill="#f8fafc" font-size="11" font-weight="700">
                          {mast.name}
                        </text>
                      {/if}
                    </g>
                  {/if}
                </g>
              {/if}
            {/each}
          {/each}
        {/if}
      {/if}
    </g>
  </svg>

  <div class="hud-overlay">
    <div class="hud-item">Zoom: {Math.round(studio.zoom * 100)}%</div>
    <div class="hud-item">Hint: Drag nodes to adjust geometry</div>
  </div>
</div>

<style>
  .canvas-container {
    flex: 1;
    position: relative;
    overflow: hidden;
    background-color: #141820;
    user-select: none;
  }

  .schematic-svg {
    width: 100%;
    height: 100%;
    cursor: grab;
  }

  .schematic-svg:active {
    cursor: grabbing;
  }

  .node-group {
    cursor: pointer;
  }

  .node-group:hover circle,
  .node-group:hover rect {
    filter: brightness(1.2);
  }

  .hud-overlay {
    position: absolute;
    bottom: 12px;
    right: 12px;
    display: flex;
    gap: 8px;
    pointer-events: none;
  }

  .hud-item {
    background: rgba(15, 23, 42, 0.85);
    backdrop-filter: blur(4px);
    color: #94a3b8;
    padding: 4px 10px;
    border-radius: 4px;
    font-size: 11px;
    border: 1px solid rgba(255, 255, 255, 0.08);
  }
</style>
