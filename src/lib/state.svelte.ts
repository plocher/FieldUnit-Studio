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
  selectedRouteId = $state<string | null>(null);
  selectedApplianceId = $state<string | null>(null);

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

  // Computed: Dynamic CP Bounding Box based on boundary IRJs and contained nodes
  cpBounds = $derived.by(() => {
    if (!this.project) {
      return { x: 220, y: 135, width: 360, height: 185, centerX: 400, bottomY: 320 };
    }

    const irjWest = this.project.graph.nodes['IRJ_WEST'];
    const irjEastMain = this.project.graph.nodes['IRJ_EAST_MAIN'];
    const irjEastSiding = this.project.graph.nodes['IRJ_EAST_SIDING'];

    const leftX = irjWest ? irjWest.x : 220;
    const rightX = Math.max(irjEastMain ? irjEastMain.x : 580, irjEastSiding ? irjEastSiding.x : 580);

    const minY = Math.min(irjWest ? irjWest.y : 180, irjEastMain ? irjEastMain.y : 180) - 45;
    const maxY = Math.max(irjWest ? irjWest.y : 180, irjEastSiding ? irjEastSiding.y : 280) + 45;

    const width = Math.max(rightX - leftX, 100);
    const height = Math.max(maxY - minY, 80);

    return {
      x: leftX,
      y: minY,
      width,
      height,
      centerX: leftX + width / 2,
      bottomY: minY + height,
    };
  });

  async loadDemo() {
    try {
      const demo = await invoke<ProjectFile>('load_demo_project');
      this.project = demo;
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

  // Node position update: moves switch points and all its connected legs as a cluster
  updateNodePosition(nodeId: string, newX: number, newY: number) {
    if (!this.project || !this.project.graph.nodes[nodeId]) return;

    const node = this.project.graph.nodes[nodeId];
    const snapX = Math.round(newX / 10) * 10;
    const snapY = Math.round(newY / 10) * 10;
    const dx = snapX - node.x;
    const dy = snapY - node.y;

    node.x = snapX;
    node.y = snapY;

    // If moving switch points, inspect the graph edges and move all outgoing terminal legs together
    if ('SwitchPoints' in node.kind) {
      for (const edge of this.project.graph.edges) {
        if (edge.from === nodeId) {
          const target = this.project.graph.nodes[edge.to];
          // Move target if it is a terminal, junction, bumper, or boundary
          if (target && !('SwitchPoints' in target.kind)) {
            target.x += dx;
            target.y += dy;
          }
        }
      }
    }
  }

  // Universal Node Snapping and Wire Splitting
  snapAndMerge(draggedId: string) {
    if (!this.project || !this.project.graph.nodes[draggedId]) return;
    const draggedNode = this.project.graph.nodes[draggedId];

    // 1. Check if dropped directly onto an existing node (snap to node)
    for (const [targetId, targetNode] of Object.entries(this.project.graph.nodes)) {
      if (targetId === draggedId) continue;
      const dist = Math.hypot(draggedNode.x - targetNode.x, draggedNode.y - targetNode.y);
      if (dist <= 26) {
        // Case A: Dragging SwitchPoints onto a Boundary (extend mainline/siding)
        if ('SwitchPoints' in draggedNode.kind && 'Boundary' in targetNode.kind) {
          // Re-route incoming edges that previously terminated at boundary to the switch points
          for (const edge of this.project.graph.edges) {
            if (edge.to === targetId) {
              edge.to = draggedId;
            }
          }

          // Move the boundary to the end of the normal leg of this switch
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

          this.selectedNodeId = draggedId;
          this.runDrc();
          this.synthesizeRoutes();
          return;
        }

        // Case B: Dragging a switch leg (trailing point) onto a Boundary or existing Track
        for (const edge of this.project.graph.edges) {
          if (edge.from === draggedId) edge.from = targetId;
          if (edge.to === draggedId) edge.to = targetId;
        }

        // If dragged node is an appliance, preserve its identity; otherwise remove dummy junction
        if (!('SwitchPoints' in draggedNode.kind || 'Irj' in draggedNode.kind || 'Boundary' in draggedNode.kind)) {
          delete this.project.graph.nodes[draggedId];
        } else {
          draggedNode.x = targetNode.x;
          draggedNode.y = targetNode.y;
        }

        this.selectedNodeId = targetId;
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

  toggleLayer(layer: keyof LayerVisibility) {
    this.layers = {
      ...this.layers,
      [layer]: !this.layers[layer],
    };
  }

  // Appliance creation from palette (click or drop)
  addAppliance(kind: string, targetX?: number, targetY?: number) {
    if (!this.project) return;

    const cp = this.project.control_points[0];
    const x = targetX ?? (400 - this.panX) / this.zoom;
    const y = targetY ?? (200 - this.panY) / this.zoom;
    const roundedX = Math.round(x / 10) * 10;
    const roundedY = Math.round(y / 10) * 10;

    const existingSwitches = cp.switches.length;
    const nextOddSwitch = existingSwitches * 2 + 1;
    const existingSignals = cp.signal_masts.length;
    const nextEvenSignal = (existingSignals + 1) * 2;

    switch (kind) {
      case 'turnout': {
        const swId = `${nextOddSwitch}`;
        const nodePtsId = `SW${swId}_PTS`;
        const nodeNormId = `SW${swId}_NORM`;
        const nodeRevBumperId = `BUMPER_SW${swId}`;

        this.project.graph.nodes[nodePtsId] = {
          id: nodePtsId,
          kind: { SwitchPoints: { switch_id: swId } },
          x: roundedX,
          y: roundedY,
        };
        this.project.graph.nodes[nodeNormId] = {
          id: nodeNormId,
          kind: { Junction: { id: `J_${swId}_N` } },
          x: roundedX + 100,
          y: roundedY,
        };
        this.project.graph.nodes[nodeRevBumperId] = {
          id: nodeRevBumperId,
          kind: { Bumper: { id: nodeRevBumperId } },
          x: roundedX + 100,
          y: roundedY + 60,
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
          motor_pin: null,
          normal_sense_pin: null,
          reverse_sense_pin: null,
          island_circuit_id: `${swId}T`,
        });
        this.selectedNodeId = nodePtsId;
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
      case 'bumper': {
        const id = `BUMPER_${Date.now().toString().slice(-4)}`;
        this.project.graph.nodes[id] = {
          id,
          kind: { Bumper: { id } },
          x: roundedX,
          y: roundedY,
        };
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
        break;
      }
    }

    this.runDrc();
  }
}

export const studio = new StudioState();
