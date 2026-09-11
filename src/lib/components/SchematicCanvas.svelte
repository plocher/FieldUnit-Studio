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
    // If middle click or space bar pressed, start panning
    if (event.button === 1 || event.button === 0 && (event.target as HTMLElement).tagName === 'svg') {
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

  function isEdgeInActiveRoute(edge: TrackEdge): boolean {
    if (!studio.activeRoute) return false;
    return studio.activeRoute.clears_circuits.some(
      (c) => edge.kind && ('Tangent' in edge.kind && edge.kind.Tangent.circuit_id === c ||
                           'SwitchNormal' in edge.kind && edge.kind.SwitchNormal.circuit_id === c ||
                           'SwitchReverse' in edge.kind && edge.kind.SwitchReverse.circuit_id === c)
    );
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="canvas-container" onwheel={handleWheel} onmousedown={handleMouseDown} onmousemove={handleMouseMove} onmouseup={handleMouseUp}>
  <svg bind:this={svgElement} class="schematic-svg">
    <defs>
      <!-- Subtle dot grid pattern -->
      <pattern id="grid-dots" width="20" height="20" patternUnits="userSpaceOnUse">
        <circle cx="2" cy="2" r="1" fill="#333b47" />
      </pattern>
      <!-- Glow filter for active route -->
      <filter id="route-glow" x="-20%" y="-20%" width="140%" height="140%">
        <feGaussianBlur stdDeviation="4" result="blur" />
        <feComposite in="SourceGraphic" in2="blur" operator="over" />
      </filter>
    </defs>

    <!-- Background Grid -->
    <rect width="100%" height="100%" fill="#181c24" />
    <g transform="translate({studio.panX}, {studio.panY}) scale({studio.zoom})">
      <rect x="-2000" y="-2000" width="4000" height="4000" fill="url(#grid-dots)" opacity="0.8" />

      {#if studio.project}
        <!-- Control Point Boundary Halo Underlay (Verified Milestone = Soft Green) -->
        <rect
          x="180"
          y="100"
          width="440"
          height="220"
          rx="12"
          fill="#10b981"
          fill-opacity="0.04"
          stroke="#10b981"
          stroke-opacity="0.3"
          stroke-width="1.5"
          stroke-dasharray="4 4"
        />
        {#if studio.layers.names}
          <text x="195" y="125" fill="#10b981" font-size="12" font-weight="600" opacity="0.8">
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
              <!-- Track shadow/bed -->
              <line
                x1={fromNode.x}
                y1={fromNode.y}
                x2={toNode.x}
                y2={toNode.y}
                stroke="#0f172a"
                stroke-width="8"
                stroke-linecap="round"
              />
              <!-- Track steel rails -->
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

              <!-- Operational Speed Tint on Turnout Branches -->
              {#if studio.layers.speeds && 'SwitchReverse' in edge.kind}
                <text
                  x={(fromNode.x + toNode.x) / 2 + 5}
                  y={(fromNode.y + toNode.y) / 2 - 5}
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

        <!-- 2. Nodes & Appliances -->
        {#each Object.values(studio.project.graph.nodes) as node}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <g
            class="node-group"
            transform="translate({node.x}, {node.y})"
            onmousedown={(e) => startNodeDrag(e, node)}
          >
            {#if 'Boundary' in node.kind}
              <!-- CP Boundary Flag -->
              <rect x="-8" y="-14" width="16" height="28" rx="2" fill="#3b82f6" fill-opacity="0.2" stroke="#60a5fa" stroke-width="1.5" />
              <line x1="0" y1="-14" x2="0" y2="14" stroke="#60a5fa" stroke-width="2" />
              {#if studio.layers.names}
                <text x="0" y="24" text-anchor="middle" fill="#93c5fd" font-size="10">
                  {node.kind.Boundary.boundary_id}
                </text>
              {/if}
            {:else if 'Irj' in node.kind}
              <!-- Insulated Rail Joint Symbol ][ -->
              {#if studio.layers.electrical}
                <line x1="-3" y1="-10" x2="-3" y2="10" stroke="#ef4444" stroke-width="2.5" />
                <line x1="3" y1="-10" x2="3" y2="10" stroke="#ef4444" stroke-width="2.5" />
                {#if studio.layers.names}
                  <text x="0" y="-14" text-anchor="middle" fill="#f87171" font-size="9" font-family="monospace">
                    {node.kind.Irj.circuit_left} ][ {node.kind.Irj.circuit_right}
                  </text>
                {/if}
              {/if}
            {:else if 'SwitchPoints' in node.kind}
              <!-- Switch Point Node -->
              <circle cx="0" cy="0" r="5" fill="#f59e0b" stroke="#ffffff" stroke-width="1.5" />
              {#if studio.layers.names}
                <text x="0" y="-12" text-anchor="middle" fill="#fbbf24" font-size="11" font-weight="bold">
                  SW1 (ODD)
                </text>
              {/if}
            {:else if 'Bumper' in node.kind}
              <!-- Track Bumper Stop -->
              <rect x="-4" y="-8" width="8" height="16" fill="#ef4444" stroke="#ffffff" stroke-width="1" />
            {/if}

            <!-- Selection Indicator -->
            {#if studio.selectedNodeId === node.id}
              <circle cx="0" cy="0" r="12" fill="none" stroke="#38bdf8" stroke-width="2" stroke-dasharray="3 3" />
            {/if}
          </g>
        {/each}

        <!-- 3. Signal Layer (Masts & Heads) -->
        {#if studio.layers.signals}
          {#each studio.project.control_points as cp}
            {#each cp.signal_masts as mast}
              {@const irjNode = mast.irj_node_id ? studio.project.graph.nodes[mast.irj_node_id] : null}
              {#if irjNode}
                {@const isRight = mast.direction === 'Right'}
                {@const mastY = isRight ? irjNode.y + 35 : irjNode.y - 35}
                <g class="signal-group" transform="translate({irjNode.x}, {irjNode.y})">
                  <!-- Mast pole -->
                  <line x1="0" y1="0" x2="0" y2={isRight ? 35 : -35} stroke="#64748b" stroke-width="2" />
                  <!-- Signal Head Mount -->
                  <g transform="translate(0, {isRight ? 35 : -35})">
                    <rect x="-6" y="-12" width="12" height="24" rx="3" fill="#0f172a" stroke="#cbd5e1" stroke-width="1.5" />
                    <circle cx="0" cy="-6" r="3.5" fill="#22c55e" />
                    {#if mast.mast_type === 'TwoHead'}
                      <circle cx="0" cy="6" r="3.5" fill="#ef4444" />
                    {/if}
                    {#if studio.layers.names}
                      <text x={isRight ? 12 : -12} y="4" text-anchor={isRight ? 'start' : 'end'} fill="#f8fafc" font-size="10" font-weight="600">
                        {mast.name}
                      </text>
                    {/if}
                  </g>
                </g>
              {/if}
            {/each}
          {/each}
        {/if}
      {/if}
    </g>
  </svg>

  <!-- Viewport HUD Overlay -->
  <div class="hud-overlay">
    <div class="hud-item">Zoom: {Math.round(studio.zoom * 100)}%</div>
    <div class="hud-item">Hint: Drag nodes to test rubberband routing</div>
  </div>
</div>

<style>
  .canvas-container {
    flex: 1;
    position: relative;
    overflow: hidden;
    background-color: #181c24;
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
    background: rgba(15, 23, 42, 0.75);
    backdrop-filter: blur(4px);
    color: #94a3b8;
    padding: 4px 10px;
    border-radius: 4px;
    font-size: 11px;
    border: 1px solid rgba(255, 255, 255, 0.08);
  }
</style>
