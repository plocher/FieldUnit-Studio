import { invoke } from '@tauri-apps/api/core';
import type {
  ProjectFile,
  LayerVisibility,
  WorkspaceMode,
  DrcViolation,
  SynthesizedRoute,
} from './types';

// Helper: Distance from a point to a line segment
function distToSegment(px: number, py: number, x1: number, y1: number, x2: number, y2: number): number {
  const l2 = (x2 - x1) * (x2 - x1) + (y2 - y1) * (y2 - y1);
  if (l2 === 0) return Math.hypot(px - x1, py - y1);
  let t = ((px - x1) * (x2 - x1) + (py - y1) * (y2 - y1)) / l2;
  t = Math.max(0, Math.min(1, t));
  return Math.hypot(px - (x1 + t * (x2 - x1)), py - (y1 + t * (y2 - y1)));
}

export class StudioState {
  project = $state<ProjectFile | null>(null);
  workspaceMode = $state<WorkspaceMode>('schematic');
  layers = $state<LayerVisibility>({
    track: true,
    electrical: true,
    signals: true,
    speeds: true,
    names: true,
  });

  // Viewport transforms (clamped zoom)
  zoom = $state<number>(1.0);
  panX = $state<number>(0);
  panY = $state<number>(0);

  // Selection
  selectedNodeId = $state<string | null>(null);
  selectedNodeIds = $state<string[]>([]);
  selectedCircuitId = $state<string | null>(null);
  selectedRouteId = $state<string | null>(null);
  selectedApplianceId = $state<string | null>(null);

  // Stateful Appliance Placement Tool (armed state from palette)
  activeTool = $state<string | null>(null);
  armedTurnoutOrientation = $state<'FacingEastDivergeDown' | 'FacingEastDivergeUp' | 'FacingWestDivergeDown' | 'FacingWestDivergeUp'>('FacingEastDivergeDown');

  // Undo / Redo history
  undoStack: string[] = [];
  redoStack: string[] = [];

  // Panels
  isLeftSidebarOpen = $state<boolean>(true);
  isRightSidebarOpen = $state<boolean>(true);
  isBottomPanelOpen = $state<boolean>(true);
  bottomTab = $state<'matrix' | 'drc'>('matrix');

  // Diagnostics
  drcViolations = $state<DrcViolation[]>([]);

  // Computed: Active selected route
  activeRoute = $derived<SynthesizedRoute | null>(
    this.project && this.selectedRouteId
      ? this.project.routes.find((r) => r.id === this.selectedRouteId) || null
      : null
  );

  // Computed: Dynamic CP Bounding Box aligned to CP boundaries and all internal plant elements
  cpBounds = $derived.by(() => {
    if (!this.project) {
      return { x: 50, y: 120, width: 720, height: 210, centerX: 410, bottomY: 330 };
    }

    const allNodes = Object.values(this.project.graph.nodes);
    if (allNodes.length === 0) {
      return { x: 50, y: 120, width: 720, height: 210, centerX: 410, bottomY: 330 };
    }

    const xs = allNodes.map((n) => n.x);
    const ys = allNodes.map((n) => n.y);

    const minX = Math.min(...xs) - 16;
    const maxX = Math.max(...xs) + 16;
    const minY = Math.min(...ys) - 45;
    const maxY = Math.max(...ys) + 45;

    const width = Math.max(maxX - minX, 140);
    const height = Math.max(maxY - minY, 100);

    return {
      x: minX,
      y: minY,
      width,
      height,
      centerX: minX + width / 2,
      bottomY: minY + height,
    };
  });

  saveSnapshot() {
    if (!this.project) return;
    this.undoStack.push(JSON.stringify(this.project));
    if (this.undoStack.length > 50) this.undoStack.shift();
    this.redoStack = [];
  }

  undo() {
    if (this.undoStack.length === 0 || !this.project) return;
    const current = JSON.stringify(this.project);
    this.redoStack.push(current);
    const previous = this.undoStack.pop()!;
    this.project = JSON.parse(previous);
    this.clearSelection();
    this.runDrc();
    this.synthesizeRoutes();
  }

  redo() {
    if (this.redoStack.length === 0 || !this.project) return;
    const current = JSON.stringify(this.project);
    this.undoStack.push(current);
    const next = this.redoStack.pop()!;
    this.project = JSON.parse(next);
    this.clearSelection();
    this.runDrc();
    this.synthesizeRoutes();
  }

  async loadDemo() {
    try {
      const demo = await invoke<ProjectFile>('load_demo_project');
      this.project = demo;
      this.clearSelection();
      this.undoStack = [];
      this.redoStack = [];
      await this.runDrc();
    } catch (err) {
      console.error('Failed to load demo project:', err);
    }
  }

  async runDrc() {
    if (!this.project) return;
    try {
      const violations = await invoke<DrcViolation[]>('run_drc', {
        graph: this.project.graph,
      });
      this.drcViolations = violations;
    } catch (err) {
      console.error('Failed to run DRC:', err);
    }
  }

  async synthesizeRoutes() {
    if (!this.project) return;
    try {
      const routes = await invoke<SynthesizedRoute[]>('synthesize_routes', {
        project: this.project,
      });
      this.project.routes = routes;
    } catch (err) {
      console.error('Failed to synthesize routes:', err);
    }
  }

  // Zoom manipulation with strict bounds
  setZoom(newZoom: number) {
    this.zoom = Math.min(Math.max(newZoom, 0.5), 2.5);
  }

  zoomIn() {
    this.setZoom(this.zoom + 0.15);
  }

  zoomOut() {
    this.setZoom(this.zoom - 0.15);
  }

  resetZoom() {
    this.zoom = 1.0;
    this.panX = 0;
    this.panY = 0;
  }

  // Selection handlers
  selectNode(nodeId: string, isShift: boolean) {
    if (isShift) {
      if (this.selectedNodeIds.includes(nodeId)) {
        this.selectedNodeIds = this.selectedNodeIds.filter((id) => id !== nodeId);
      } else {
        this.selectedNodeIds = [...this.selectedNodeIds, nodeId];
      }
      this.selectedNodeId = this.selectedNodeIds[this.selectedNodeIds.length - 1] || null;
    } else {
      this.selectedNodeId = nodeId;
      this.selectedNodeIds = [nodeId];
    }
  }

  selectNodesInBox(x1: number, y1: number, x2: number, y2: number) {
    if (!this.project) return;
    const minX = Math.min(x1, x2);
    const maxX = Math.max(x1, x2);
    const minY = Math.min(y1, y2);
    const maxY = Math.max(y1, y2);

    const matches: string[] = [];
    for (const [id, node] of Object.entries(this.project.graph.nodes)) {
      if (node.x >= minX && node.x <= maxX && node.y >= minY && node.y <= maxY) {
        matches.push(id);
      }
    }
    this.selectedNodeIds = matches;
    this.selectedNodeId = matches[0] || null;
  }

  clearSelection() {
    this.selectedNodeId = null;
    this.selectedNodeIds = [];
    this.selectedCircuitId = null;
  }

  // Select entire Detection Block and all its bounding/endpoint nodes
  selectCircuit(circuitId: string) {
    if (!this.project) return;
    this.selectedCircuitId = circuitId;

    const nodeSet = new Set<string>();
    for (const edge of this.project.graph.edges) {
      const cId = 'Tangent' in edge.kind ? edge.kind.Tangent.circuit_id :
                  'SwitchNormal' in edge.kind ? edge.kind.SwitchNormal.circuit_id :
                  'SwitchReverse' in edge.kind ? edge.kind.SwitchReverse.circuit_id : '';
      if (cId === circuitId) {
        nodeSet.add(edge.from);
        nodeSet.add(edge.to);
      }
    }

    this.selectedNodeIds = Array.from(nodeSet);
    this.selectedNodeId = this.selectedNodeIds[0] || null;
  }

  // KiCad-style 'U' key: extend selection along the SAME corridor level, stopping at level changes
  extendSelection(allowLevelChange: boolean = false) {
    if (!this.project) return;
    const current = new Set(
      this.selectedNodeIds.length > 0
        ? this.selectedNodeIds
        : this.selectedNodeId
        ? [this.selectedNodeId]
        : []
    );
    if (current.size === 0) return;

    const next = new Set(current);
    for (const edge of this.project.graph.edges) {
      const fromNode = this.project.graph.nodes[edge.from];
      const toNode = this.project.graph.nodes[edge.to];
      if (!fromNode || !toNode) continue;

      // Stop at corridor level changes unless allowLevelChange is true
      const isSameLevel = Math.abs(fromNode.y - toNode.y) <= 15;

      if (isSameLevel || allowLevelChange) {
        if (current.has(edge.from)) next.add(edge.to);
        if (current.has(edge.to)) next.add(edge.from);
      }
    }
    this.selectedNodeIds = Array.from(next);
    if (!this.selectedNodeId && this.selectedNodeIds.length > 0) {
      this.selectedNodeId = this.selectedNodeIds[0];
    }
  }

  // Move node(s): enforces discrete corridor lane snapping (50px increments)
  updateNodePosition(nodeId: string, newX: number, newY: number) {
    if (!this.project || !this.project.graph.nodes[nodeId]) return;

    const node = this.project.graph.nodes[nodeId];
    const snapX = Math.round(newX / 10) * 10;
    // Snap Y to discrete corridor lanes centered on y = 180 with 50px lane spacing
    const snapY = Math.round((newY - 80) / 50) * 50 + 80;

    const dx = snapX - node.x;
    const dy = snapY - node.y;

    if (this.selectedNodeIds.length > 1 && this.selectedNodeIds.includes(nodeId)) {
      for (const id of this.selectedNodeIds) {
        const n = this.project.graph.nodes[id];
        if (n) {
          n.x += dx;
          n.y += dy;
        }
      }
      return;
    }

    node.x = snapX;
    node.y = snapY;

    // If moving switch points, move all connected outgoing terminal legs together
    if ('SwitchPoints' in node.kind) {
      for (const edge of this.project.graph.edges) {
        if (edge.from === nodeId) {
          const target = this.project.graph.nodes[edge.to];
          if (target && !('SwitchPoints' in target.kind)) {
            target.x += dx;
            target.y += dy;
          }
        }
      }
    }
  }

  // Delete selected nodes, heal tracks when deleting IRJs/switches, and remove orphaned terminals
  deleteSelected() {
    if (!this.project) return;
    const idsToDelete = this.selectedNodeIds.length > 0
      ? [...this.selectedNodeIds]
      : this.selectedNodeId
      ? [this.selectedNodeId]
      : [];

    if (idsToDelete.length === 0) return;
    this.saveSnapshot();
    const cp = this.project.control_points[0];

    for (const id of idsToDelete) {
      const node = this.project.graph.nodes[id];
      if (!node) continue;

      if ('SwitchPoints' in node.kind) {
        const swId = node.kind.SwitchPoints.switch_id;
        cp.switches = cp.switches.filter((s) => s.id !== swId);

        // Delete unattached terminals created for this switch
        delete this.project.graph.nodes[`SW${swId}_NORM`];
        delete this.project.graph.nodes[`BUMPER_SW${swId}`];

        // Heal track: connect incoming edge directly to downstream normal edge
        const incomingEdge = this.project.graph.edges.find((e) => e.to === id);
        const outgoingNormEdge = this.project.graph.edges.find((e) => e.from === id && 'SwitchNormal' in e.kind);
        if (incomingEdge && outgoingNormEdge) {
          incomingEdge.to = outgoingNormEdge.to;
        }
      }

      if ('Irj' in node.kind) {
        cp.signal_masts = cp.signal_masts.filter((m) => m.irj_node_id !== id);

        // Heal track across deleted IRJ: connect incoming edge directly to downstream edge
        const incomingEdge = this.project.graph.edges.find((e) => e.to === id);
        const outgoingEdge = this.project.graph.edges.find((e) => e.from === id);
        if (incomingEdge && outgoingEdge) {
          incomingEdge.to = outgoingEdge.to;
          // Drop the redundant outgoing edge so track is continuous
          this.project.graph.edges = this.project.graph.edges.filter((e) => e.id !== outgoingEdge.id);
        }
      }

      delete this.project.graph.nodes[id];
    }

    // Remove remaining edges connected to deleted nodes
    this.project.graph.edges = this.project.graph.edges.filter(
      (e) => !idsToDelete.includes(e.from) && !idsToDelete.includes(e.to)
    );

    this.clearSelection();
    this.renumberAppliancesWestToEast();
    this.runDrc();
    this.synthesizeRoutes();
  }

  // Renumber all switches West to East sequentially (odd numbers: 1, 3, 5, 7...)
  renumberAppliancesWestToEast() {
    if (!this.project) return;
    const cp = this.project.control_points[0];

    const swItems = cp.switches.map((sw) => {
      const nodeEntry = Object.entries(this.project!.graph.nodes).find(
        ([_, n]) => 'SwitchPoints' in n.kind && n.kind.SwitchPoints.switch_id === sw.id
      );
      return {
        sw,
        nodeId: nodeEntry ? nodeEntry[0] : null,
        x: nodeEntry ? nodeEntry[1].x : 0,
      };
    });

    swItems.sort((a, b) => a.x - b.x);

    swItems.forEach((item, index) => {
      const newId = `${index * 2 + 1}`;
      const oldId = item.sw.id;
      if (oldId !== newId) {
        item.sw.id = newId;
        item.sw.name = newId;

        if (item.nodeId && this.project!.graph.nodes[item.nodeId]) {
          this.project!.graph.nodes[item.nodeId].kind = {
            SwitchPoints: { switch_id: newId },
          };
        }

        for (const edge of this.project!.graph.edges) {
          if ('SwitchNormal' in edge.kind && edge.kind.SwitchNormal.switch_id === oldId) {
            edge.kind.SwitchNormal.switch_id = newId;
          }
          if ('SwitchReverse' in edge.kind && edge.kind.SwitchReverse.switch_id === oldId) {
            edge.kind.SwitchReverse.switch_id = newId;
          }
        }
      }
    });
  }

  // Insert a new Turnout inline on an existing track edge
  insertTurnoutOnEdge(edgeIndex: number, x: number, y: number): string | null {
    if (!this.project || edgeIndex < 0 || edgeIndex >= this.project.graph.edges.length) return null;
    this.saveSnapshot();

    const edge = this.project.graph.edges[edgeIndex];
    const fromNode = this.project.graph.nodes[edge.from];
    const toNode = this.project.graph.nodes[edge.to];
    if (!fromNode || !toNode) return null;

    // Project click position accurately onto the edge vector
    const l2 = (toNode.x - fromNode.x) ** 2 + (toNode.y - fromNode.y) ** 2;
    let t = l2 === 0 ? 0.5 : ((x - fromNode.x) * (toNode.x - fromNode.x) + (y - fromNode.y) * (toNode.y - fromNode.y)) / l2;
    t = Math.max(0.2, Math.min(0.8, t));

    const snapX = Math.round((fromNode.x + t * (toNode.x - fromNode.x)) / 10) * 10;
    const snapY = Math.round((fromNode.y + t * (toNode.y - fromNode.y)) / 10) * 10;
    const shiftDistance = 140;

    // Shift only downstream nodes connected after toNode
    for (const node of Object.values(this.project.graph.nodes)) {
      if (node.x >= snapX + 10 && node.id !== toNode.id) {
        node.x += shiftDistance;
      }
    }
    toNode.x += shiftDistance;

    const cp = this.project.control_points[0];
    const tempId = `${Date.now().toString().slice(-4)}`;
    const ptsId = `SW${tempId}_PTS`;
    const revBumperId = `BUMPER_SW${tempId}`;

    // Switch points inserted directly on the track line
    this.project.graph.nodes[ptsId] = {
      id: ptsId,
      kind: { SwitchPoints: { switch_id: tempId } },
      x: snapX,
      y: snapY,
    };

    // Diverging reverse branch with buffer stop
    this.project.graph.nodes[revBumperId] = {
      id: revBumperId,
      kind: { Bumper: { id: revBumperId } },
      x: snapX + 100,
      y: snapY + 50,
    };

    // Original incoming track terminates at new switch points
    const originalTo = edge.to;
    edge.to = ptsId;

    // Normal switch branch connects from points to the original downstream track
    this.project.graph.edges.push({
      id: `E_SW${tempId}_NORM`,
      from: ptsId,
      to: originalTo,
      kind: { SwitchNormal: { switch_id: tempId, circuit_id: '1T' } },
      length_feet: 100,
    });

    // Reverse switch branch connects points to the bumper
    this.project.graph.edges.push({
      id: `E_SW${tempId}_REV`,
      from: ptsId,
      to: revBumperId,
      kind: { SwitchReverse: { switch_id: tempId, circuit_id: 'SPUR', speed: 'Slow' } },
      length_feet: 120,
    });

    cp.switches.push({
      id: tempId,
      name: tempId,
      speed: 'Slow',
      motor_pin: null,
      normal_sense_pin: null,
      reverse_sense_pin: null,
      island_circuit_id: '1T',
    });

    this.selectNode(ptsId, false);
    this.renumberAppliancesWestToEast();
    this.runDrc();
    this.synthesizeRoutes();
    return ptsId;
  }

  // Cleanly split an existing track edge and insert an IRJ node with graceful horizontal expansion
  insertIrjOnEdge(edgeIndex: number, x: number, y: number): string | null {
    if (!this.project || edgeIndex < 0 || edgeIndex >= this.project.graph.edges.length) return null;
    this.saveSnapshot();

    const edge = this.project.graph.edges[edgeIndex];
    let snapX = Math.round(x / 10) * 10;
    const snapY = Math.round(y / 10) * 10;
    const shiftDistance = 140;

    // Enforce minimum distance from any existing IRJ node (at least 60px) to prevent crowded joints
    for (const node of Object.values(this.project.graph.nodes)) {
      if ('Irj' in node.kind && Math.abs(snapX - node.x) < 60) {
        snapX = node.x + 70;
      }
    }

    // Graceful horizontal expansion: shift all downstream nodes to the right to make room
    for (const node of Object.values(this.project.graph.nodes)) {
      if (node.x >= snapX) {
        node.x += shiftDistance;
      }
    }

    const irjId = `IRJ_${Date.now().toString().slice(-4)}`;
    this.project.graph.nodes[irjId] = {
      id: irjId,
      kind: { Irj: { id: irjId, circuit_left: '1T', circuit_right: '2T' } },
      x: snapX,
      y: snapY,
    };

    const originalTo = edge.to;
    edge.to = irjId;

    this.project.graph.edges.push({
      id: `E_SPLIT_${Date.now().toString().slice(-4)}`,
      from: irjId,
      to: originalTo,
      kind: edge.kind,
      length_feet: edge.length_feet / 2,
    });

    this.selectNode(irjId, false);
    this.runDrc();
    this.synthesizeRoutes();
    return irjId;
  }

  // Universal Node Snapping and Wire Splitting
  snapAndMerge(draggedId: string) {
    if (!this.project || !this.project.graph.nodes[draggedId]) return;
    const draggedNode = this.project.graph.nodes[draggedId];

    // 1. Check if dropped directly onto an existing node (snap to node)
    for (const [targetId, targetNode] of Object.entries(this.project.graph.nodes)) {
      if (targetId === draggedId) continue;

      // Safety checks: never merge two IRJs or nodes that are already direct neighbors
      if ('Irj' in draggedNode.kind && 'Irj' in targetNode.kind) continue;
      const alreadyConnected = this.project.graph.edges.some(
        (e) => (e.from === draggedId && e.to === targetId) || (e.from === targetId && e.to === draggedId)
      );
      if (alreadyConnected) continue;

      const dist = Math.hypot(draggedNode.x - targetNode.x, draggedNode.y - targetNode.y);
      if (dist <= 36) {
        // Case A: Dragging SwitchPoints onto a Boundary (extend mainline/siding)
        if ('SwitchPoints' in draggedNode.kind && 'Boundary' in targetNode.kind) {
          for (const edge of this.project.graph.edges) {
            if (edge.to === targetId) {
              edge.to = draggedId;
            }
          }

          for (const edge of this.project.graph.edges) {
            if (edge.from === draggedId && 'SwitchNormal' in edge.kind) {
              const normTerminal = this.project.graph.nodes[edge.to];
              if (normTerminal) {
                targetNode.x = normTerminal.x;
                targetNode.y = normTerminal.y;
                edge.to = targetId;
                delete this.project.graph.nodes[normTerminal.id];
              }
              break;
            }
          }

          this.selectNode(draggedId, false);
          this.renumberAppliancesWestToEast();
          this.runDrc();
          this.synthesizeRoutes();
          return;
        }

        // Case B: Dragging or stamping SwitchPoints onto a Bumper (extend spur into switch)
        if ('SwitchPoints' in draggedNode.kind && 'Bumper' in targetNode.kind) {
          for (const edge of this.project.graph.edges) {
            if (edge.to === targetId) {
              edge.to = draggedId;
            }
            if (edge.from === targetId) {
              edge.from = draggedId;
            }
          }
          delete this.project.graph.nodes[targetId];

          draggedNode.x = targetNode.x;
          draggedNode.y = targetNode.y;

          this.selectNode(draggedId, false);
          this.renumberAppliancesWestToEast();
          this.runDrc();
          this.synthesizeRoutes();
          return;
        }

        // Case C: Dropping SwitchPoints near an IRJ (signals bind to IRJ; connect switch downstream of IRJ)
        if ('SwitchPoints' in draggedNode.kind && 'Irj' in targetNode.kind) {
          this.project.graph.edges.push({
            id: `E_CONN_${Date.now().toString().slice(-4)}`,
            from: targetId,
            to: draggedId,
            kind: { Tangent: { circuit_id: '1T' } },
            length_feet: 40,
          });

          draggedNode.x = targetNode.x + 50;
          draggedNode.y = targetNode.y;

          this.selectNode(draggedId, false);
          this.renumberAppliancesWestToEast();
          this.runDrc();
          this.synthesizeRoutes();
          return;
        }
        // Case D: Dragging a dummy junction onto an existing node
        if ('Junction' in draggedNode.kind) {
          for (const edge of this.project.graph.edges) {
            if (edge.from === draggedId) edge.from = targetId;
            if (edge.to === draggedId) edge.to = targetId;
          }
          delete this.project.graph.nodes[draggedId];
          this.selectNode(targetId, false);
          this.runDrc();
          this.synthesizeRoutes();
          return;
        }

        this.selectNode(targetId, false);
        this.runDrc();
        this.synthesizeRoutes();
        return;
      }
    }

    // 2. Check if dropped onto a track edge line (split edge to insert inline node, e.g. IRJ)
    if ('Irj' in draggedNode.kind) {
      for (let i = 0; i < this.project.graph.edges.length; i++) {
        const edge = this.project.graph.edges[i];
        const fromNode = this.project.graph.nodes[edge.from];
        const toNode = this.project.graph.nodes[edge.to];
        if (!fromNode || !toNode) continue;

        const dist = distToSegment(draggedNode.x, draggedNode.y, fromNode.x, fromNode.y, toNode.x, toNode.y);
        if (dist <= 14) {
          // Snap IRJ onto the edge line
          draggedNode.y = fromNode.y; // Align with horizontal track

          // Split edge: edge becomes (fromNode -> draggedNode), add (draggedNode -> toNode)
          const originalTo = edge.to;
          edge.to = draggedId;

          this.project.graph.edges.push({
            id: `E_SPLIT_${Date.now().toString().slice(-4)}`,
            from: draggedId,
            to: originalTo,
            kind: edge.kind,
            length_feet: edge.length_feet / 2,
          });

          this.runDrc();
          this.synthesizeRoutes();
          return;
        }
      }
    }
  }

  // Flip a switch diverge side (Diverge Up <-> Diverge Down)
  flipSelectedSwitch() {
    if (!this.project) return;

    if (this.activeTool === 'turnout') {
      const flipMap = {
        FacingEastDivergeDown: 'FacingEastDivergeUp',
        FacingEastDivergeUp: 'FacingEastDivergeDown',
        FacingWestDivergeDown: 'FacingWestDivergeUp',
        FacingWestDivergeUp: 'FacingWestDivergeDown',
      } as const;
      this.armedTurnoutOrientation = flipMap[this.armedTurnoutOrientation];
      return;
    }

    const cp = this.project.control_points[0];
    let targetSwId: string | null = null;

    if (this.selectedNodeId && this.project.graph.nodes[this.selectedNodeId]) {
      const node = this.project.graph.nodes[this.selectedNodeId];
      if ('SwitchPoints' in node.kind) targetSwId = node.kind.SwitchPoints.switch_id;
    }
    if (!targetSwId) {
      for (const id of this.selectedNodeIds) {
        const node = this.project.graph.nodes[id];
        if (node && 'SwitchPoints' in node.kind) {
          targetSwId = node.kind.SwitchPoints.switch_id;
          break;
        }
      }
    }
    if (!targetSwId) return;
    this.saveSnapshot();

    const sw = cp.switches.find((s) => s.id === targetSwId);
    if (!sw) return;

    const currentOrient = sw.orientation || 'FacingEastDivergeDown';
    const isCurrentlyUp = currentOrient.includes('Up');
    const nextOrient = isCurrentlyUp
      ? (currentOrient.replace('Up', 'Down') as typeof currentOrient)
      : (currentOrient.replace('Down', 'Up') as typeof currentOrient);
    sw.orientation = nextOrient;

    const ptsNodeEntry = Object.entries(this.project.graph.nodes).find(
      ([_, n]) => 'SwitchPoints' in n.kind && n.kind.SwitchPoints.switch_id === targetSwId
    );
    if (!ptsNodeEntry) return;
    const [ptsId, ptsNode] = ptsNodeEntry;

    // Flip reverse branch vertically (if was Up, go Down; if was Down, go Up)
    const targetDy = isCurrentlyUp ? 60 : -60;

    for (const edge of this.project.graph.edges) {
      if (edge.from === ptsId && 'SwitchReverse' in edge.kind) {
        const revNode = this.project.graph.nodes[edge.to];
        if (revNode) {
          const newY = ptsNode.y + targetDy;
          const deltaY = newY - revNode.y;
          revNode.y = newY;

          // Shift any connected bumpers or terminals on this reverse branch
          for (const subEdge of this.project.graph.edges) {
            if (subEdge.from === revNode.id) {
              const child = this.project.graph.nodes[subEdge.to];
              if (child && !('SwitchPoints' in child.kind)) {
                child.y += deltaY;
              }
            }
          }
        }
      }
    }

    this.runDrc();
    this.synthesizeRoutes();
  }

  // Rotate a switch facing direction by 180° (Facing East <-> Facing West / Trailing)
  rotateSelectedSwitch() {
    if (!this.project) return;

    if (this.activeTool === 'turnout') {
      const rotMap = {
        FacingEastDivergeDown: 'FacingWestDivergeDown',
        FacingEastDivergeUp: 'FacingWestDivergeUp',
        FacingWestDivergeDown: 'FacingEastDivergeDown',
        FacingWestDivergeUp: 'FacingEastDivergeUp',
      } as const;
      this.armedTurnoutOrientation = rotMap[this.armedTurnoutOrientation];
      return;
    }

    const cp = this.project.control_points[0];
    let targetSwId: string | null = null;

    if (this.selectedNodeId && this.project.graph.nodes[this.selectedNodeId]) {
      const node = this.project.graph.nodes[this.selectedNodeId];
      if ('SwitchPoints' in node.kind) targetSwId = node.kind.SwitchPoints.switch_id;
    }
    if (!targetSwId) {
      for (const id of this.selectedNodeIds) {
        const node = this.project.graph.nodes[id];
        if (node && 'SwitchPoints' in node.kind) {
          targetSwId = node.kind.SwitchPoints.switch_id;
          break;
        }
      }
    }
    if (!targetSwId) return;
    this.saveSnapshot();

    const sw = cp.switches.find((s) => s.id === targetSwId);
    if (!sw) return;

    const currentOrient = sw.orientation || 'FacingEastDivergeDown';
    const isCurrentlyEast = currentOrient.includes('East');
    const nextOrient = isCurrentlyEast
      ? (currentOrient.replace('East', 'West') as typeof currentOrient)
      : (currentOrient.replace('West', 'East') as typeof currentOrient);
    sw.orientation = nextOrient;

    const ptsNodeEntry = Object.entries(this.project.graph.nodes).find(
      ([_, n]) => 'SwitchPoints' in n.kind && n.kind.SwitchPoints.switch_id === targetSwId
    );
    if (!ptsNodeEntry) return;
    const [ptsId, ptsNode] = ptsNodeEntry;

    // Flip branches horizontally (East -> West or West -> East)
    for (const edge of this.project.graph.edges) {
      if (edge.from === ptsId) {
        const target = this.project.graph.nodes[edge.to];
        if (target && !('SwitchPoints' in target.kind)) {
          const currentDx = target.x - ptsNode.x;
          const targetDx = isCurrentlyEast ? -Math.abs(currentDx || 100) : Math.abs(currentDx || 100);
          const deltaX = (ptsNode.x + targetDx) - target.x;
          target.x = ptsNode.x + targetDx;

          // Shift downstream bumpers or terminals
          for (const subEdge of this.project.graph.edges) {
            if (subEdge.from === target.id) {
              const child = this.project.graph.nodes[subEdge.to];
              if (child && !('SwitchPoints' in child.kind)) {
                child.x += deltaX;
              }
            }
          }
        }
      }
    }

    this.runDrc();
    this.synthesizeRoutes();
  }

  // Tidy / Auto-Layout model board layout to exact AAR standards (0°, 45°, 90°)
  autoArrange() {
    if (!this.project) return;
    this.saveSnapshot();

    // 1. Align main line to y = 180 and siding tracks to y = 280
    for (const [id, node] of Object.entries(this.project.graph.nodes)) {
      if (id.includes('SIDING') || id.includes('REV') || id.includes('BUMPER')) {
        node.y = 280;
      } else {
        node.y = 180;
      }
      node.x = Math.round(node.x / 20) * 20;
    }

    // 2. Ensure reverse branches maintain exact 45° diverge geometry
    for (const edge of this.project.graph.edges) {
      if ('SwitchReverse' in edge.kind) {
        const fromNode = this.project.graph.nodes[edge.from];
        const toNode = this.project.graph.nodes[edge.to];
        if (fromNode && toNode && fromNode.y === 180 && toNode.y === 280) {
          toNode.x = fromNode.x + 100; // Exact 45° angle: dx = 100, dy = 100
        }
      }
    }

    this.runDrc();
    this.synthesizeRoutes();
  }

  toggleLayer(layer: keyof LayerVisibility) {
    this.layers = {
      ...this.layers,
      [layer]: !this.layers[layer],
    };
  }

  // Appliance creation from palette (click or drop)
  addAppliance(kind: string, targetX?: number, targetY?: number): string | null {
    if (!this.project) return null;
    this.saveSnapshot();

    const cp = this.project.control_points[0];
    const x = targetX ?? (420 - this.panX) / this.zoom;
    const y = targetY ?? (220 - this.panY) / this.zoom;
    const roundedX = Math.round(x / 10) * 10;
    const roundedY = Math.round(y / 10) * 10;

    const existingSwitches = cp.switches.length;
    const nextOddSwitch = existingSwitches * 2 + 1;
    const existingSignals = cp.signal_masts.length;
    const nextEvenSignal = (existingSignals + 1) * 2;

    let createdId: string | null = null;

    switch (kind) {
      case 'turnout': {
        const swId = `${nextOddSwitch}`;
        const nodePtsId = `SW${swId}_PTS`;
        const nodeNormId = `SW${swId}_NORM`;
        const nodeRevBumperId = `BUMPER_SW${swId}`;

        // Calculate branch vectors based on armed turnout orientation
        let normDx = 100, normDy = 0;
        let revDx = 100, revDy = 60;
        switch (this.armedTurnoutOrientation) {
          case 'FacingEastDivergeDown':
            normDx = 100; normDy = 0; revDx = 100; revDy = 60; break;
          case 'FacingEastDivergeUp':
            normDx = 100; normDy = 0; revDx = 100; revDy = -60; break;
          case 'FacingWestDivergeDown':
            normDx = -100; normDy = 0; revDx = -100; revDy = 60; break;
          case 'FacingWestDivergeUp':
            normDx = -100; normDy = 0; revDx = -100; revDy = -60; break;
        }

        this.project.graph.nodes[nodePtsId] = {
          id: nodePtsId,
          kind: { SwitchPoints: { switch_id: swId } },
          x: roundedX,
          y: roundedY,
        };
        this.project.graph.nodes[nodeNormId] = {
          id: nodeNormId,
          kind: { Junction: { id: `J_${swId}_N` } },
          x: roundedX + normDx,
          y: roundedY + normDy,
        };
        this.project.graph.nodes[nodeRevBumperId] = {
          id: nodeRevBumperId,
          kind: { Bumper: { id: nodeRevBumperId } },
          x: roundedX + revDx,
          y: roundedY + revDy,
        };

        this.project.graph.edges.push({
          id: `E_SW${swId}_NORM`,
          from: nodePtsId,
          to: nodeNormId,
          kind: { SwitchNormal: { switch_id: swId, circuit_id: `${swId}T` } },
          length_feet: 100,
        });
        this.project.graph.edges.push({
          id: `E_SW${swId}_REV`,
          from: nodePtsId,
          to: nodeRevBumperId,
          kind: { SwitchReverse: { switch_id: swId, circuit_id: `${swId}T`, speed: 'Slow' } },
          length_feet: 120,
        });

        cp.switches.push({
          id: swId,
          name: swId,
          speed: 'Slow',
          orientation: this.armedTurnoutOrientation,
          motor_pin: null,
          normal_sense_pin: null,
          reverse_sense_pin: null,
          island_circuit_id: `${swId}T`,
        });
        createdId = nodePtsId;
        break;
      }
      case 'irj': {
        const id = `IRJ_${Date.now().toString().slice(-4)}`;
        this.project.graph.nodes[id] = {
          id,
          kind: { Irj: { id, circuit_left: '1T', circuit_right: '2T' } },
          x: roundedX,
          y: roundedY,
        };
        createdId = id;
        break;
      }
      case 'signal': {
        const sigId = `${nextEvenSignal}Sab`;
        const irjNodes = Object.keys(this.project.graph.nodes).filter((k) => k.includes('IRJ'));
        const targetIrj = irjNodes[0] || null;

        cp.signal_masts.push({
          id: sigId,
          name: sigId,
          mast_type: 'TwoHead',
          direction: 'Right',
          governing_lever: `${nextEvenSignal}`,
          irj_node_id: targetIrj,
        });
        break;
      }
      case 'block': {
        // Detection Block: creates a track segment bounded by an IRJ
        const irjId = `IRJ_${Date.now().toString().slice(-4)}`;
        const termId = `TERM_${Date.now().toString().slice(-4)}`;
        const blockId = `${existingSwitches + 1}T`;

        this.project.graph.nodes[irjId] = {
          id: irjId,
          kind: { Irj: { id: irjId, circuit_left: `${blockId}A`, circuit_right: blockId } },
          x: roundedX,
          y: roundedY,
        };
        this.project.graph.nodes[termId] = {
          id: termId,
          kind: { Junction: { id: termId } },
          x: roundedX + 120,
          y: roundedY,
        };

        this.project.graph.edges.push({
          id: `E_${blockId}`,
          from: irjId,
          to: termId,
          kind: { Tangent: { circuit_id: blockId } },
          length_feet: 200,
        });

        cp.track_circuits.push({
          id: blockId,
          name: blockId,
          is_island: false,
          dropout_delay_ms: 100,
          sensor_pin: null,
          optical_pin: null,
        });
        this.selectedNodeIds = [irjId, termId];
        this.selectedNodeId = irjId;
        createdId = irjId;
        break;
      }
      case 'bumper': {
        const id = `BUMPER_${Date.now().toString().slice(-4)}`;
        this.project.graph.nodes[id] = {
          id,
          kind: { Bumper: { id } },
          x: roundedX,
          y: roundedY,
        };
        createdId = id;
        break;
      }
      case 'boundary': {
        const id = `B_${Date.now().toString().slice(-4)}`;
        this.project.graph.nodes[id] = {
          id,
          kind: { Boundary: { boundary_id: id, direction: 'Right' } },
          x: roundedX,
          y: roundedY,
        };
        createdId = id;
        break;
      }
    }

    if (createdId) {
      this.selectNode(createdId, false);
    }
    this.runDrc();
    return createdId;
  }
}

export const studio = new StudioState();
