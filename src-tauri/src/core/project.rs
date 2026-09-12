use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use super::graph::{EdgeKind, NodeKind, TrackEdge, TrackGraph, TrackNode};
use super::model::{
    BoundaryType, ControlPoint, CpBoundary, Direction, Indication, MastType, SignalMast,
    SpeedClass, Switch, SwitchPosition, TrackCircuit,
};
use super::route_synthesizer::SynthesizedRoute;

/// Project metadata header.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectMetadata {
    pub name: String,
    pub subdivision: String,
    pub rulebook: String,
    pub version: String,
    pub author: Option<String>,
}

impl Default for ProjectMetadata {
    fn default() -> Self {
        Self {
            name: "New Subdivision".to_string(),
            subdivision: "Main Line".to_string(),
            rulebook: "GCOR".to_string(),
            version: "1.0.0".to_string(),
            author: None,
        }
    }
}

/// The unified .fieldunit project persistence bundle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFile {
    pub metadata: ProjectMetadata,
    pub control_points: Vec<ControlPoint>,
    pub graph: TrackGraph,
    pub routes: Vec<SynthesizedRoute>,
}

impl ProjectFile {
    pub fn new(name: impl Into<String>, subdivision: impl Into<String>) -> Self {
        Self {
            metadata: ProjectMetadata {
                name: name.into(),
                subdivision: subdivision.into(),
                rulebook: "GCOR".to_string(),
                version: "1.0.0".to_string(),
                author: None,
            },
            control_points: Vec::new(),
            graph: TrackGraph::new(),
            routes: Vec::new(),
        }
    }

    /// Serializes the project to formatted JSON text.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserializes a project from JSON text.
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }

    /// Saves the project to a file on disk.
    pub fn save_to_file(&self, path: &Path) -> std::io::Result<()> {
        let json = self.to_json().map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(path, json)
    }

    /// Loads a project from a file on disk.
    pub fn load_from_file(path: &Path) -> std::io::Result<Self> {
        let json = fs::read_to_string(path)?;
        Self::from_json(&json).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Imports a plant directly from FieldUnit's standardized plant JSON schema (Issue #1 / CP_Corporal.json).
    pub fn from_plant_json(json_str: &str) -> Result<Self, serde_json::Error> {
        let plant: FieldUnitPlantJson = serde_json::from_str(json_str)?;
        let mut project = Self::new(&plant.name, "Coast Division");
        let mut cp = ControlPoint::new(&plant.name, &plant.name, "SP Coast Line MP 83");

        // Track Circuits
        for tc in &plant.track_circuits {
            let is_island = plant.detector_locks.as_ref().map_or(false, |locks| {
                locks.iter().any(|l| l.track_circuit == tc.name)
            });
            let mut circuit = TrackCircuit::new(&tc.name, &tc.name, is_island);
            if let Some(delay) = tc.dropout_delay_ms {
                circuit.dropout_delay_ms = delay;
            }
            cp.track_circuits.push(circuit);
        }

        // Switches
        for sw in &plant.switches {
            let mut s = Switch::new(&sw.name, &sw.name);
            if let Some(locks) = &plant.detector_locks {
                if let Some(lock) = locks.iter().find(|l| l.switch == sw.name) {
                    s.island_circuit_id = Some(lock.track_circuit.clone());
                }
            }
            cp.switches.push(s);
        }

        // Signal Masts
        for sm in &plant.signal_masts {
            let mast_type = match sm.r#type.as_str() {
                "TWO_HEAD" => MastType::TwoHead,
                "THREE_HEAD" => MastType::ThreeHead,
                "DWARF" => MastType::Dwarf,
                _ => MastType::OneHead,
            };
            let dir = if sm.name.contains("SA") {
                Direction::Right
            } else {
                Direction::Left
            };
            let mut mast = SignalMast::new(&sm.name, &sm.name, mast_type, dir);
            // Associate to respective island/approach IRJ
            if sm.name == "2NAB" {
                mast.irj_node_id = Some("IRJ_3T1_EAST".to_string());
            } else if sm.name == "2SA" {
                mast.irj_node_id = Some("IRJ_1SAT_WEST".to_string());
            } else if sm.name == "4NA" {
                mast.irj_node_id = Some("IRJ_5T1_EAST".to_string());
            } else if sm.name == "4SA" {
                mast.irj_node_id = Some("IRJ_2SAT_WEST".to_string());
            }
            cp.signal_masts.push(mast);
        }

        // Boundaries
        cp.boundaries.push(CpBoundary {
            id: "B_WEST_MT2".to_string(),
            name: "West MT2 (2SAT)".to_string(),
            direction: Direction::Left,
            boundary_type: BoundaryType::Abs,
            connected_track_id: Some("2SAT".to_string()),
        });
        cp.boundaries.push(CpBoundary {
            id: "B_WEST_MT1".to_string(),
            name: "West MT1 (1SAT)".to_string(),
            direction: Direction::Left,
            boundary_type: BoundaryType::Abs,
            connected_track_id: Some("1SAT".to_string()),
        });
        cp.boundaries.push(CpBoundary {
            id: "B_EAST_SINGLE".to_string(),
            name: "East Single (1NAT/2NAT)".to_string(),
            direction: Direction::Right,
            boundary_type: BoundaryType::Abs,
            connected_track_id: Some("1NAT".to_string()),
        });

        // Build topological graph
        let mut graph = TrackGraph::new();

        // Level 0: MT2 Mainline
        graph.add_node(TrackNode {
            id: "B_WEST_MT2".to_string(),
            kind: NodeKind::Boundary { boundary_id: "WEST_MT2".to_string(), direction: Direction::Left },
            x: 80.0,
            y: 180.0,
        });
        graph.add_node(TrackNode {
            id: "IRJ_2SAT_WEST".to_string(),
            kind: NodeKind::Irj { id: "IRJ_2SAT".to_string(), circuit_left: "2SAT".to_string(), circuit_right: "1T1".to_string() },
            x: 220.0,
            y: 180.0,
        });
        graph.add_node(TrackNode {
            id: "SW1_PTS".to_string(),
            kind: NodeKind::SwitchPoints { switch_id: "1".to_string() },
            x: 360.0,
            y: 180.0,
        });
        graph.add_node(TrackNode {
            id: "SW3_PTS".to_string(),
            kind: NodeKind::SwitchPoints { switch_id: "3".to_string() },
            x: 580.0,
            y: 180.0,
        });
        graph.add_node(TrackNode {
            id: "IRJ_3T1_EAST".to_string(),
            kind: NodeKind::Irj { id: "IRJ_3T1".to_string(), circuit_left: "3T1".to_string(), circuit_right: "1NAT".to_string() },
            x: 740.0,
            y: 180.0,
        });
        graph.add_node(TrackNode {
            id: "B_EAST_SINGLE".to_string(),
            kind: NodeKind::Boundary { boundary_id: "EAST_SINGLE".to_string(), direction: Direction::Right },
            x: 880.0,
            y: 180.0,
        });

        // Level 1: MT1 Southbound Main
        graph.add_node(TrackNode {
            id: "B_WEST_MT1".to_string(),
            kind: NodeKind::Boundary { boundary_id: "WEST_MT1".to_string(), direction: Direction::Left },
            x: 80.0,
            y: 280.0,
        });
        graph.add_node(TrackNode {
            id: "IRJ_1SAT_WEST".to_string(),
            kind: NodeKind::Irj { id: "IRJ_1SAT".to_string(), circuit_left: "1SAT".to_string(), circuit_right: "3T1".to_string() },
            x: 220.0,
            y: 280.0,
        });
        graph.add_node(TrackNode {
            id: "MT1_MERGE_POINT".to_string(),
            kind: NodeKind::Junction { id: "J_MT1_MERGE".to_string() },
            x: 480.0,
            y: 280.0,
        });

        // Level -1: Industry Lead & Derail 5
        graph.add_node(TrackNode {
            id: "SW5_PTS".to_string(),
            kind: NodeKind::SwitchPoints { switch_id: "5".to_string() },
            x: 460.0,
            y: 80.0,
        });
        graph.add_node(TrackNode {
            id: "IRJ_5T1_EAST".to_string(),
            kind: NodeKind::Irj { id: "IRJ_5T1".to_string(), circuit_left: "5T1".to_string(), circuit_right: "IND1".to_string() },
            x: 600.0,
            y: 80.0,
        });
        graph.add_node(TrackNode {
            id: "IND_BUMPER".to_string(),
            kind: NodeKind::Bumper { id: "B_BEET_LOADER".to_string() },
            x: 800.0,
            y: 80.0,
        });

        // Edges
        graph.add_edge(TrackEdge {
            id: "E_2SAT".to_string(),
            from: "B_WEST_MT2".to_string(),
            to: "IRJ_2SAT_WEST".to_string(),
            kind: EdgeKind::Tangent { circuit_id: "2SAT".to_string() },
            length_feet: 500.0,
        });
        graph.add_edge(TrackEdge {
            id: "E_1T1_ENTRY".to_string(),
            from: "IRJ_2SAT_WEST".to_string(),
            to: "SW1_PTS".to_string(),
            kind: EdgeKind::Tangent { circuit_id: "1T1".to_string() },
            length_feet: 60.0,
        });
        graph.add_edge(TrackEdge {
            id: "E_SW1_NORM".to_string(),
            from: "SW1_PTS".to_string(),
            to: "SW3_PTS".to_string(),
            kind: EdgeKind::SwitchNormal { switch_id: "1".to_string(), circuit_id: "1T1".to_string() },
            length_feet: 140.0,
        });
        graph.add_edge(TrackEdge {
            id: "E_SW3_NORM".to_string(),
            from: "SW3_PTS".to_string(),
            to: "IRJ_3T1_EAST".to_string(),
            kind: EdgeKind::SwitchNormal { switch_id: "3".to_string(), circuit_id: "3T1".to_string() },
            length_feet: 100.0,
        });
        graph.add_edge(TrackEdge {
            id: "E_1NAT".to_string(),
            from: "IRJ_3T1_EAST".to_string(),
            to: "B_EAST_SINGLE".to_string(),
            kind: EdgeKind::Tangent { circuit_id: "1NAT".to_string() },
            length_feet: 500.0,
        });

        graph.add_edge(TrackEdge {
            id: "E_1SAT".to_string(),
            from: "B_WEST_MT1".to_string(),
            to: "IRJ_1SAT_WEST".to_string(),
            kind: EdgeKind::Tangent { circuit_id: "1SAT".to_string() },
            length_feet: 500.0,
        });
        graph.add_edge(TrackEdge {
            id: "E_MT1_RUN".to_string(),
            from: "IRJ_1SAT_WEST".to_string(),
            to: "MT1_MERGE_POINT".to_string(),
            kind: EdgeKind::Tangent { circuit_id: "3T1".to_string() },
            length_feet: 200.0,
        });
        graph.add_edge(TrackEdge {
            id: "E_SW3_REV".to_string(),
            from: "SW3_PTS".to_string(),
            to: "MT1_MERGE_POINT".to_string(),
            kind: EdgeKind::SwitchReverse { switch_id: "3".to_string(), circuit_id: "3T1".to_string(), speed: SpeedClass::Medium },
            length_feet: 140.0,
        });

        graph.add_edge(TrackEdge {
            id: "E_SW1_REV".to_string(),
            from: "SW1_PTS".to_string(),
            to: "SW5_PTS".to_string(),
            kind: EdgeKind::SwitchReverse { switch_id: "1".to_string(), circuit_id: "5T1".to_string(), speed: SpeedClass::Slow },
            length_feet: 140.0,
        });
        graph.add_edge(TrackEdge {
            id: "E_SW5_NORM".to_string(),
            from: "SW5_PTS".to_string(),
            to: "IRJ_5T1_EAST".to_string(),
            kind: EdgeKind::SwitchNormal { switch_id: "5".to_string(), circuit_id: "5T1".to_string() },
            length_feet: 80.0,
        });
        graph.add_edge(TrackEdge {
            id: "E_IND_SPUR".to_string(),
            from: "IRJ_5T1_EAST".to_string(),
            to: "IND_BUMPER".to_string(),
            kind: EdgeKind::Tangent { circuit_id: "IND1".to_string() },
            length_feet: 300.0,
        });

        // Convert routes from JSON
        let mut routes = Vec::new();
        for r in &plant.routes {
            let mut alignments = HashMap::new();
            for a in &r.aligns {
                let pos = if a.position == "REVERSE" { SwitchPosition::Reverse } else { SwitchPosition::Normal };
                alignments.insert(a.switch.clone(), pos);
            }
            let dir = if r.governed_by.direction == "RIGHT" { Direction::Right } else { Direction::Left };
            let ceiling = match r.displays.as_ref().map(|d| d.max_indication.as_str()).unwrap_or("CLEAR") {
                "RESTRICTING" => Indication::Restricting,
                "DIVERGING_RESTRICTING" => Indication::SlowClear,
                "DIVERGING_CLEAR" => Indication::DivergingClear,
                _ => Indication::Clear,
            };

            routes.push(SynthesizedRoute {
                id: format!("RT_{}", r.name),
                name: r.name.clone(),
                entrance_signal_id: r.governed_by.signal.clone(),
                exit_node_id: r.clears.last().cloned().unwrap_or_default(),
                direction: dir,
                switch_alignments: alignments,
                clears_circuits: r.clears.clone(),
                aspect_ceiling: ceiling,
                fleeting_capable: r.aligns.iter().all(|a| a.position == "NORMAL"),
                call_on_capable: r.aligns.iter().any(|a| a.position == "REVERSE"),
                conflicts_with: Vec::new(),
                concurrent_with: Vec::new(),
            });
        }

        project.control_points.push(cp);
        project.graph = graph;
        project.routes = routes;

        Ok(project)
    }
}

/// Raw FieldUnit plant JSON schema matching Issue #1 / PlantSerializer.h
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldUnitPlantJson {
    pub name: String,
    pub default_aspect_policy: Option<String>,
    pub track_circuits: Vec<PlantJsonTrackCircuit>,
    pub switches: Vec<PlantJsonSwitch>,
    pub crossovers: Option<Vec<PlantJsonCrossover>>,
    pub signal_controls: Option<Vec<PlantJsonSignalControl>>,
    pub signal_masts: Vec<PlantJsonSignalMast>,
    pub detector_locks: Option<Vec<PlantJsonDetectorLock>>,
    pub routes: Vec<PlantJsonRoute>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlantJsonTrackCircuit {
    pub name: String,
    pub dropout_delay_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlantJsonSwitch {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlantJsonCrossover {
    pub name: String,
    pub switch_a: Option<String>,
    pub switch_b: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlantJsonSignalControl {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlantJsonSignalMast {
    pub name: String,
    pub r#type: String,
    pub aspect_policy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlantJsonDetectorLock {
    pub switch: String,
    pub track_circuit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlantJsonRoute {
    pub name: String,
    pub governed_by: PlantJsonGovernedBy,
    pub displays: Option<PlantJsonDisplays>,
    pub aligns: Vec<PlantJsonAlign>,
    pub clears: Vec<String>,
    pub entrance: Option<String>,
    pub approaching: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlantJsonGovernedBy {
    pub signal: String,
    pub direction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlantJsonDisplays {
    pub mast: String,
    pub head: Option<u8>,
    pub max_indication: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlantJsonAlign {
    pub switch: String,
    pub position: String,
}
