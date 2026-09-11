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

  // Computed
  activeRoute = $derived<SynthesizedRoute | null>(
    this.project && this.selectedRouteId
      ? this.project.routes.find((r) => r.id === this.selectedRouteId) || null
      : null
  );

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

  // Node position update (rubberbanding)
  updateNodePosition(nodeId: string, x: number, y: number) {
    if (this.project && this.project.graph.nodes[nodeId]) {
      this.project.graph.nodes[nodeId].x = Math.round(x / 10) * 10;
      this.project.graph.nodes[nodeId].y = Math.round(y / 10) * 10;
    }
  }

  toggleLayer(layer: keyof LayerVisibility) {
    this.layers[layer] = !this.layers[layer];
  }
}

export const studio = new StudioState();
