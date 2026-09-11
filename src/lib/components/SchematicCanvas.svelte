<script lang="ts">
  import { studio } from '$lib/state.svelte';
  import type { TrackNode, TrackEdge } from '$lib/types';

  let svgElement: SVGSVGElement | null = $state(null);
  let isPanning = $state(false);
  let panStartX = $state(0);
  let panStartY = $state(0);

  // Marquee Bounding Box Selection
  let isMarquee = $state(false);
  let marqueeStart = $state({ x: 0, y: 0 });
  let marqueeCurrent = $state({ x: 0, y: 0 });

  let draggingNodeId = $state<string | null>(null);
  let dragOffset = $state({ x: 0, y: 0 });

  // Smooth Edge Auto-Pan via requestAnimationFrame
  let panVelocity = { x: 0, y: 0 };
  let animFrameId: number | null = null;

  function startAutoPanLoop() {
    if (animFrameId !== null) return;
    function loop() {
      if (panVelocity.x !== 0 || panVelocity.y !== 0) {
        studio.panX += panVelocity.x;
        studio.panY += panVelocity.y;
      }
      animFrameId = requestAnimationFrame(loop);
    }
    animFrameId = requestAnimationFrame(loop);
  }

  function stopAutoPanLoop() {
    if (animFrameId !== null) {
      cancelAnimationFrame(animFrameId);
      animFrameId = null;
    }
    panVelocity = { x: 0, y: 0 };
  }

  function handleWheel(event: WheelEvent) {
    event.preventDefault();
    const factor = event.deltaY < 0 ? 1.08 : 0.92;
    studio.setZoom(studio.zoom * factor);
  }

  function handleMouseDown(event: MouseEvent) {
    // If clicking on a node group, node drag handler takes over
    if ((event.target as HTMLElement).closest('.node-group')) {
      return;
    }

    const rect = svgElement?.getBoundingClientRect();
    if (!rect) return;
    const canvasX = (event.clientX - rect.left - studio.panX) / studio.zoom;
    const canvasY = (event.clientY - rect.top - studio.panY) / studio.zoom;

    // Shift + Left Click on background: Start Marquee Selection Box
    if (event.shiftKey && event.button === 0) {
      isMarquee = true;
      marqueeStart = { x: canvasX, y: canvasY };
      marqueeCurrent = { x: canvasX, y: canvasY };
      return;
    }

    // Normal Left Click or Middle Click on background: Pan Canvas
    if (event.button === 0 || event.button === 1) {
      isPanning = true;
      panStartX = event.clientX - studio.panX;
      panStartY = event.clientY - studio.panY;
      studio.clearSelection();
    }
  }

  function handleMouseMove(event: MouseEvent) {
    const rect = svgElement?.getBoundingClientRect();
    if (!rect) return;

    if (isPanning) {
      studio.panX = event.clientX - panStartX;
      studio.panY = event.clientY - panStartY;
    } else if (isMarquee) {
      const canvasX = (event.clientX - rect.left - studio.panX) / studio.zoom;
      const canvasY = (event.clientY - rect.top - studio.panY) / studio.zoom;
      marqueeCurrent = { x: canvasX, y: canvasY };
    } else if (draggingNodeId && studio.project) {
      // Auto-pan viewport when dragging near edges
      const margin = 60;
      const speed = 12;
      let vx = 0;
      let vy = 0;

      if (event.clientX < rect.left + margin) {
        vx = speed;
      } else if (event.clientX > rect.right - margin) {
        vx = -speed;
      }
      if (event.clientY < rect.top + margin) {
        vy = speed;
      } else if (event.clientY > rect.bottom - margin) {
        vy = -speed;
      }

      panVelocity = { x: vx, y: vy };
      if (vx !== 0 || vy !== 0) {
        startAutoPanLoop();
      } else {
        stopAutoPanLoop();
      }

      const mouseX = (event.clientX - rect.left - studio.panX) / studio.zoom;
      const mouseY = (event.clientY - rect.top - studio.panY) / studio.zoom;
      studio.updateNodePosition(draggingNodeId, mouseX - dragOffset.x, mouseY - dragOffset.y);
    }
  }

  function handleMouseUp() {
    stopAutoPanLoop();

    if (isMarquee) {
      studio.selectNodesInBox(marqueeStart.x, marqueeStart.y, marqueeCurrent.x, marqueeCurrent.y);
      isMarquee = false;
    } else if (draggingNodeId) {
      studio.snapAndMerge(draggingNodeId);
      draggingNodeId = null;
    }

    isPanning = false;
  }

  function startNodeDrag(event: MouseEvent, node: TrackNode) {
    event.stopPropagation();
    draggingNodeId = node.id;
    studio.selectNode(node.id, event.shiftKey);

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
        <!-- Dynamic Control Point Boundary Box -->
        <!-- Automatically updates size and position when boundary IRJs move -->
        <rect
          x={studio.cpBounds.x}
          y={studio.cpBounds.y}
          width={studio.cpBounds.width}
          height={studio.cpBounds.height}
          rx="8"
          fill="#10b981"
          fill-opacity="0.04"
          stroke="#10b981"
          stroke-opacity="0.35"
          stroke-width="1.5"
          stroke-dasharray="6 4"
        />
        <!-- Centered CP Name at Bottom of CP Box -->
        <g opacity={studio.layers.names ? 1.0 : 0.3}>
          <text
            x={studio.cpBounds.centerX}
            y={studio.cpBounds.bottomY - 10}
            text-anchor="middle"
            fill="#10b981"
            font-size="11"
            font-weight="700"
            letter-spacing="0.5"
          >
            {studio.project.control_points[0]?.name.toUpperCase() || 'CP'}
          </text>
        </g>

        <!-- 1. Track Layer (Edges / Rails) - Bright or Dimmed -->
        <g opacity={studio.layers.track ? 1.0 : 0.3}>
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

              <!-- Block Names in clean dark bubble with RED text (only on primary straight segments, not duplicated) -->
              <g opacity={studio.layers.electrical ? 1.0 : 0.3}>
                {#if circuitId && (edge.id === 'E_APP' || edge.id === 'E_NORM' || edge.id === 'E_EXIT_MAIN' || edge.id === 'E_EXIT_SIDING')}
                  <g transform="translate({midX}, {midY})">
                    <rect x="-16" y="-8" width="32" height="16" rx="3" fill="#0f172a" stroke="#334155" stroke-width="1" />
                    <text x="0" y="4" text-anchor="middle" fill="#ef4444" font-size="10" font-family="monospace" font-weight="700">
                      {circuitId}
                    </text>
                  </g>
                {/if}
              </g>
            {/if}
          {/each}
        </g>

        <!-- 2. Nodes (IRJ, Switch Points, Boundaries, Junctions) -->
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
              <g opacity={studio.layers.names ? 1.0 : 0.3}>
                <text x="0" y="24" text-anchor="middle" fill="#93c5fd" font-size="10" font-weight="600">
                  {node.kind.Boundary.boundary_id}
                </text>
              </g>
            {:else if 'Irj' in node.kind}
              <!-- Insulated Rail Joint Symbol ][ -->
              <g opacity={studio.layers.electrical ? 1.0 : 0.3}>
                <line x1="-3" y1="-10" x2="-3" y2="10" stroke="#ef4444" stroke-width="2.5" />
                <line x1="3" y1="-10" x2="3" y2="10" stroke="#ef4444" stroke-width="2.5" />
              </g>
            {:else if 'SwitchPoints' in node.kind}
              <!-- Switch Points Node: Subtle speed colored aura and clean identifier "1 [MED]" -->
              <g opacity={studio.layers.speeds ? 1.0 : 0.3}>
                <circle cx="0" cy="0" r="14" fill="#f59e0b" fill-opacity="0.2" />
              </g>
              <circle cx="0" cy="0" r="5" fill="#f59e0b" stroke="#ffffff" stroke-width="1.5" />
              <g opacity={studio.layers.names ? 1.0 : 0.3}>
                <text x="0" y="-12" text-anchor="middle" fill="#fbbf24" font-size="11" font-weight="bold">
                  {node.kind.SwitchPoints.switch_id}
                  <tspan fill="#f59e0b" font-size="9" font-weight="600" opacity={studio.layers.speeds ? 1.0 : 0.3}>[MED]</tspan>
                </text>
              </g>
            {:else if 'Junction' in node.kind}
              <!-- Draggable track leg terminal/junction -->
              <circle cx="0" cy="0" r="4" fill="#38bdf8" stroke="#ffffff" stroke-width="1" />
            {:else if 'Bumper' in node.kind}
              <rect x="-4" y="-8" width="8" height="16" fill="#ef4444" stroke="#ffffff" stroke-width="1" />
            {/if}

            <!-- Selection Indicator for single or multi-selected nodes -->
            {#if studio.selectedNodeIds.includes(node.id) || studio.selectedNodeId === node.id}
              <circle cx="0" cy="0" r="14" fill="none" stroke="#38bdf8" stroke-width="2" stroke-dasharray="3 3" />
            {/if}
          </g>
        {/each}

        <!-- 3. Signal Layer (Horizontal Heads with Symmetrical Base Centered on Mast Arm) -->
        <g opacity={studio.layers.signals ? 1.0 : 0.3}>
          {#each studio.project.control_points as cp}
            {#each cp.signal_masts as mast}
              {@const irjNode = mast.irj_node_id ? studio.project.graph.nodes[mast.irj_node_id] : null}
              {#if irjNode}
                {@const isRight = mast.direction === 'Right'}
                <g class="signal-mast-group">
                  {#if isRight}
                    <!-- 2Sab: Below rail, base centered on arm, arm extending right, heads horizontal: |--oo -->
                    <g transform="translate({irjNode.x}, {irjNode.y})">
                      <!-- Symmetrical Base | centered on mast arm at y=22, height=16, detached from rail -->
                      <line x1="0" y1="14" x2="0" y2="30" stroke="#cbd5e1" stroke-width="2" stroke-linecap="round" />
                      <!-- Horizontal arm extending right from base center (0, 22) to capsule (14, 22) -->
                      <line x1="0" y1="22" x2="14" y2="22" stroke="#cbd5e1" stroke-width="2" />

                      <!-- Signal Body Capsule (height=16, matches base height) with white/silver border -->
                      <rect x="14" y="14" width="24" height="16" rx="4" fill="#0f172a" stroke="#e2e8f0" stroke-width="1.2" />
                      <!-- Horizontal heads: 2Sb (left) and 2Sa (right) -->
                      <circle cx="20" cy="22" r="3.5" fill="#ef4444" stroke="#000000" stroke-width="0.8" />
                      <circle cx="30" cy="22" r="3.5" fill="#22c55e" stroke="#000000" stroke-width="0.8" />

                      <!-- Signal Name cleanly below capsule, no overlap -->
                      <g opacity={studio.layers.names ? 1.0 : 0.3}>
                        <text x="26" y="44" text-anchor="middle" fill="#f8fafc" font-size="10" font-weight="700">
                          {mast.name}
                        </text>
                      </g>
                    </g>
                  {:else}
                    <!-- 2Nab / 2Nc: Above rail, heads horizontal, arm extending left, base centered: oo--| -->
                    <g transform="translate({irjNode.x}, {irjNode.y})">
                      <!-- Symmetrical Base | centered on mast arm at y=-22, height=16, detached from rail -->
                      <line x1="0" y1="-30" x2="0" y2="-14" stroke="#cbd5e1" stroke-width="2" stroke-linecap="round" />
                      <!-- Horizontal arm extending left from base center (0, -22) to capsule (-14, -22) -->
                      <line x1="0" y1="-22" x2="-14" y2="-22" stroke="#cbd5e1" stroke-width="2" />

                      <!-- Signal Body Capsule with white/silver border -->
                      {#if mast.mast_type === 'TwoHead'}
                        <rect x="-38" y="-30" width="24" height="16" rx="4" fill="#0f172a" stroke="#e2e8f0" stroke-width="1.2" />
                        <circle cx="-32" cy="-22" r="3.5" fill="#22c55e" stroke="#000000" stroke-width="0.8" />
                        <circle cx="-22" cy="-22" r="3.5" fill="#ef4444" stroke="#000000" stroke-width="0.8" />
                      {:else}
                        <!-- Dwarf signal (single head) -->
                        <rect x="-26" y="-30" width="16" height="16" rx="4" fill="#0f172a" stroke="#e2e8f0" stroke-width="1.2" />
                        <circle cx="-18" cy="-22" r="3.5" fill="#ef4444" stroke="#000000" stroke-width="0.8" />
                      {/if}

                      <!-- Signal Name cleanly above capsule, no overlap -->
                      <g opacity={studio.layers.names ? 1.0 : 0.3}>
                        <text x={mast.mast_type === 'TwoHead' ? -26 : -18} y="-36" text-anchor="middle" fill="#f8fafc" font-size="10" font-weight="700">
                          {mast.name}
                        </text>
                      </g>
                    </g>
                  {/if}
                </g>
              {/if}
            {/each}
          {/each}
        </g>
        <!-- Marquee Selection Rectangle -->
        {#if isMarquee}
          <rect
            x={Math.min(marqueeStart.x, marqueeCurrent.x)}
            y={Math.min(marqueeStart.y, marqueeCurrent.y)}
            width={Math.abs(marqueeCurrent.x - marqueeStart.x)}
            height={Math.abs(marqueeCurrent.y - marqueeStart.y)}
            fill="rgba(56, 189, 248, 0.12)"
            stroke="#38bdf8"
            stroke-width="1.5"
            stroke-dasharray="4 4"
          />
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
