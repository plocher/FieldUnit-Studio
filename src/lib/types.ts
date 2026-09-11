export type SpeedClass = 'Slow' | 'Medium' | 'Limited' | 'Normal';
export type Direction = 'Left' | 'Right';
export type SwitchPosition = 'Normal' | 'Reverse';
export type Indication =
  | 'Stop'
  | 'Restricting'
  | 'SlowClear'
  | 'DivergingClear'
  | 'LimitedClear'
  | 'Approach'
  | 'Clear';

export interface Switch {
  id: string;
  name: string;
  speed: SpeedClass;
  motor_pin?: number | null;
  normal_sense_pin?: number | null;
  reverse_sense_pin?: number | null;
  island_circuit_id?: string | null;
}

export interface Crossover {
  id: string;
  name: string;
  speed: SpeedClass;
  switch_a_id: string;
  switch_b_id: string;
}

export interface TrackCircuit {
  id: string;
  name: string;
  is_island: boolean;
  dropout_delay_ms: number;
  sensor_pin?: number | null;
  optical_pin?: number | null;
}

export type MastType = 'OneHead' | 'TwoHead' | 'ThreeHead' | 'Dwarf';

export interface SignalMast {
  id: string;
  name: string;
  mast_type: MastType;
  direction: Direction;
  governing_lever?: string | null;
  irj_node_id?: string | null;
}

export type BoundaryType = 'Abs' | 'Apb' | 'Ctc' | 'Dark';

export interface CpBoundary {
  id: string;
  name: string;
  direction: Direction;
  boundary_type: BoundaryType;
  connected_track_id?: string | null;
}

export interface ControlPoint {
  id: string;
  name: string;
  subdivision: string;
  milepost?: number | null;
  switches: Switch[];
  crossovers: Crossover[];
  track_circuits: TrackCircuit[];
  signal_masts: SignalMast[];
  boundaries: CpBoundary[];
}

export type NodeKind =
  | { Boundary: { boundary_id: string; direction: Direction } }
  | { Irj: { id: string; circuit_left: string; circuit_right: string } }
  | { SwitchPoints: { switch_id: string } }
  | { Bumper: { id: string } }
  | { Junction: { id: string } };

export interface TrackNode {
  id: string;
  kind: NodeKind;
  x: number;
  y: number;
}

export type EdgeKind =
  | { Tangent: { circuit_id: string } }
  | { SwitchNormal: { switch_id: string; circuit_id: string } }
  | { SwitchReverse: { switch_id: string; circuit_id: string; speed: SpeedClass } };

export interface TrackEdge {
  id: string;
  from: string;
  to: string;
  kind: EdgeKind;
  length_feet: number;
}

export interface TrackGraph {
  nodes: Record<string, TrackNode>;
  edges: TrackEdge[];
}

export type DrcSeverity = 'Draft' | 'Warning' | 'Conflict';

export interface DrcViolation {
  id: string;
  severity: DrcSeverity;
  message: string;
  node_id?: string | null;
}

export interface SynthesizedRoute {
  id: string;
  name: string;
  entrance_signal_id: string;
  exit_node_id: string;
  direction: Direction;
  switch_alignments: Record<string, SwitchPosition>;
  clears_circuits: string[];
  aspect_ceiling: Indication;
  fleeting_capable: boolean;
  call_on_capable: boolean;
  conflicts_with: string[];
  concurrent_with: string[];
}

export interface ProjectMetadata {
  name: string;
  subdivision: string;
  rulebook: string;
  version: string;
  author?: string | null;
}

export interface ProjectFile {
  metadata: ProjectMetadata;
  control_points: ControlPoint[];
  graph: TrackGraph;
  routes: SynthesizedRoute[];
}

export interface LayerVisibility {
  track: boolean;
  electrical: boolean;
  signals: boolean;
  speeds: boolean;
  names: boolean;
}

export type WorkspaceMode = 'schematic' | 'matrix' | 'ctc' | 'code';
