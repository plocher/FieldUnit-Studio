import { invoke } from '@tauri-apps/api/core';
import type {
  ProjectFile,
  LayerVisibility,
  WorkspaceMode,
  DrcViolation,
  SynthesizedRoute,
} from './types';

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

  // Node position update with cluster moving for switch appliances
  updateNodePosition(nodeId: string, newX: number, newY: number) {
    if (!this.project || !this.project.graph.nodes[nodeId]) return;

    const node = this.project.graph.nodes[nodeId];
    const snapX = Math.round(newX / 10) * 10;
    const snapY = Math.round(newY / 10) * 10;
    const dx = snapX - node.x;
    const dy = snapY - node.y;

    node.x = snapX;
    node.y = snapY;

    // If moving switch points, move the connected leg nodes together as a cluster
    if ('SwitchPoints' in node.kind) {
      const swId = node.kind.SwitchPoints.switch_id;
      const normLeg = this.project.graph.nodes[`SW${swId}_NORM`];
      const revLeg = this.project.graph.nodes[`SW${swId}_REV`];
      if (normLeg) {
        normLeg.x += dx;
        normLeg.y += dy;
      }
      if (revLeg) {
        revLeg.x += dx;
        revLeg.y += dy;
      }
    }
  }

  // Snap and merge dragged node into a nearby target node (within 22px)
  snapAndMerge(draggedId: string) {
    if (!this.project || !this.project.graph.nodes[draggedId]) return;
    const draggedNode = this.project.graph.nodes[draggedId];

    // Find a nearby candidate node to snap to
    for (const [targetId, targetNode] of Object.entries(this.project.graph.nodes)) {
      if (targetId === draggedId) continue;
      const dist = Math.hypot(draggedNode.x - targetNode.x, draggedNode.y - targetNode.y);
      if (dist <= 22) {
        // Snap to target coordinates
        draggedNode.x = targetNode.x;
        draggedNode.y = targetNode.y;

        // Re-route all edges from/to draggedId to targetId
        for (const edge of this.project.graph.edges) {
          if (edge.from === draggedId) {
            edge.from = targetId;
          }
          if (edge.to === draggedId) {
            edge.to = targetId;
          }
        }

        // Delete the duplicate dragged node
        delete this.project.graph.nodes[draggedId];
        this.selectedNodeId = targetId;
        this.runDrc();
        this.synthesizeRoutes();
        break;
      }
    }
  }

  toggleLayer(layer: keyof LayerVisibility) {
    this.layers[layer] = !this.layers[layer];
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
        const nodeRevId = `SW${swId}_REV`;

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
        this.project.graph.nodes[nodeRevId] = {
          id: nodeRevId,
          kind: { Junction: { id: `J_${swId}_R` } },
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
          to: nodeRevId,
          kind: { SwitchReverse: { switch_id: swId, circuit_id: `${swId}T`, speed: 'Medium' } },
          length_feet: 120,
        });

        cp.switches.push({
          id: swId,
          name: swId,
          speed: 'Medium',
          motor_pin: null,
          normal_sense_pin: null,
          reverse_sense_pin: null,
          island_circuit_id: `${swId}T`,
        });
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
