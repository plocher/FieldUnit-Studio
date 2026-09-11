pub mod core;

use std::collections::HashMap;
use core::graph::{DrcViolation, EdgeKind, NodeKind, TrackEdge, TrackGraph, TrackNode};
use core::model::{
    BoundaryType, ControlPoint, CpBoundary, Direction, MastType, SignalMast, SpeedClass,
    Switch, TrackCircuit,
};
use core::project::ProjectFile;
use core::route_synthesizer::{RouteSynthesizer, SynthesizedRoute};

/// Loads the standard Tutorial 1 Control Point End-of-Siding demo project.
#[tauri::command]
fn load_demo_project() -> Result<ProjectFile, String> {
    let mut project = ProjectFile::new("Coast Division Demo", "Santa Cruz Branch");
    let mut cp = ControlPoint::new("CP_END_SIDING", "CP End of Siding", "Santa Cruz Branch");

    // Add appliances
    cp.switches.push(Switch::new("SW1", "Switch 1"));
    cp.track_circuits.push(TrackCircuit::new("1T1", "Island 1T1", true));
    cp.track_circuits.push(TrackCircuit::new("1SA", "Approach West", false));
    cp.track_circuits.push(TrackCircuit::new("1NA", "Main Exit East", false));
    cp.track_circuits.push(TrackCircuit::new("2NA", "Siding Exit East", false));

    let mut mast2r = SignalMast::new("2R", "Signal 2R", MastType::TwoHead, Direction::Right);
    mast2r.irj_node_id = Some("IRJ_WEST".to_string());
    cp.signal_masts.push(mast2r);

    let mut mast2la = SignalMast::new("2LA", "Signal 2LA", MastType::Dwarf, Direction::Left);
    mast2la.irj_node_id = Some("IRJ_EAST_MAIN".to_string());
    cp.signal_masts.push(mast2la);

    let mut mast2lb = SignalMast::new("2LB", "Signal 2LB", MastType::Dwarf, Direction::Left);
    mast2lb.irj_node_id = Some("IRJ_EAST_SIDING".to_string());
    cp.signal_masts.push(mast2lb);

    cp.boundaries.push(CpBoundary {
        id: "B_WEST".to_string(),
        name: "West Boundary (1SA)".to_string(),
        direction: Direction::Left,
        boundary_type: BoundaryType::Abs,
        connected_track_id: Some("1SA".to_string()),
    });
    cp.boundaries.push(CpBoundary {
        id: "B_EAST_MAIN".to_string(),
        name: "East Main Boundary (1NA)".to_string(),
        direction: Direction::Right,
        boundary_type: BoundaryType::Abs,
        connected_track_id: Some("1NA".to_string()),
    });
    cp.boundaries.push(CpBoundary {
        id: "B_EAST_SIDING".to_string(),
        name: "East Siding Boundary (2NA)".to_string(),
        direction: Direction::Right,
        boundary_type: BoundaryType::Dark,
        connected_track_id: Some("2NA".to_string()),
    });

    // Build the schematic graph geometry
    let mut graph = TrackGraph::new();

    // Nodes
    graph.add_node(TrackNode {
        id: "B_WEST".to_string(),
        kind: NodeKind::Boundary { boundary_id: "WEST".to_string(), direction: Direction::Left },
        x: 60.0,
        y: 180.0,
    });
    graph.add_node(TrackNode {
        id: "IRJ_WEST".to_string(),
        kind: NodeKind::Irj {
            id: "IRJ_1".to_string(),
            circuit_left: "1SA".to_string(),
            circuit_right: "1T1".to_string(),
        },
        x: 220.0,
        y: 180.0,
    });
    graph.add_node(TrackNode {
        id: "SW1_PTS".to_string(),
        kind: NodeKind::SwitchPoints { switch_id: "SW1".to_string() },
        x: 360.0,
        y: 180.0,
    });
    graph.add_node(TrackNode {
        id: "IRJ_EAST_MAIN".to_string(),
        kind: NodeKind::Irj {
            id: "IRJ_2".to_string(),
            circuit_left: "1T1".to_string(),
            circuit_right: "1NA".to_string(),
        },
        x: 580.0,
        y: 180.0,
    });
    graph.add_node(TrackNode {
        id: "IRJ_EAST_SIDING".to_string(),
        kind: NodeKind::Irj {
            id: "IRJ_3".to_string(),
            circuit_left: "1T1".to_string(),
            circuit_right: "2NA".to_string(),
        },
        x: 580.0,
        y: 280.0,
    });
    graph.add_node(TrackNode {
        id: "B_EAST_MAIN".to_string(),
        kind: NodeKind::Boundary { boundary_id: "EAST_MAIN".to_string(), direction: Direction::Right },
        x: 740.0,
        y: 180.0,
    });
    graph.add_node(TrackNode {
        id: "B_EAST_SIDING".to_string(),
        kind: NodeKind::Boundary { boundary_id: "EAST_SIDING".to_string(), direction: Direction::Right },
        x: 740.0,
        y: 280.0,
    });

    // Edges
    graph.add_edge(TrackEdge {
        id: "E_APP".to_string(),
        from: "B_WEST".to_string(),
        to: "IRJ_WEST".to_string(),
        kind: EdgeKind::Tangent { circuit_id: "1SA".to_string() },
        length_feet: 500.0,
    });
    graph.add_edge(TrackEdge {
        id: "E_ISLAND_ENTRY".to_string(),
        from: "IRJ_WEST".to_string(),
        to: "SW1_PTS".to_string(),
        kind: EdgeKind::Tangent { circuit_id: "1T1".to_string() },
        length_feet: 60.0,
    });
    graph.add_edge(TrackEdge {
        id: "E_NORM".to_string(),
        from: "SW1_PTS".to_string(),
        to: "IRJ_EAST_MAIN".to_string(),
        kind: EdgeKind::SwitchNormal { switch_id: "SW1".to_string(), circuit_id: "1T1".to_string() },
        length_feet: 120.0,
    });
    graph.add_edge(TrackEdge {
        id: "E_REV".to_string(),
        from: "SW1_PTS".to_string(),
        to: "IRJ_EAST_SIDING".to_string(),
        kind: EdgeKind::SwitchReverse {
            switch_id: "SW1".to_string(),
            circuit_id: "1T1".to_string(),
            speed: SpeedClass::Medium,
        },
        length_feet: 140.0,
    });
    graph.add_edge(TrackEdge {
        id: "E_EXIT_MAIN".to_string(),
        from: "IRJ_EAST_MAIN".to_string(),
        to: "B_EAST_MAIN".to_string(),
        kind: EdgeKind::Tangent { circuit_id: "1NA".to_string() },
        length_feet: 500.0,
    });
    graph.add_edge(TrackEdge {
        id: "E_EXIT_SIDING".to_string(),
        from: "IRJ_EAST_SIDING".to_string(),
        to: "B_EAST_SIDING".to_string(),
        kind: EdgeKind::Tangent { circuit_id: "2NA".to_string() },
        length_feet: 500.0,
    });

    // Synthesize initial routes
    let synthesizer = RouteSynthesizer::new(&cp, &graph);
    project.routes = synthesizer.synthesize_all_routes();
    project.control_points.push(cp);
    project.graph = graph;

    Ok(project)
}

/// Synthesizes routes across a given project bundle.
#[tauri::command]
fn synthesize_routes(project: ProjectFile) -> Result<Vec<SynthesizedRoute>, String> {
    if let Some(cp) = project.control_points.first() {
        let synthesizer = RouteSynthesizer::new(cp, &project.graph);
        Ok(synthesizer.synthesize_all_routes())
    } else {
        Ok(Vec::new())
    }
}

/// Executes Design Rule Checks on a track graph.
#[tauri::command]
fn run_drc(graph: TrackGraph) -> Result<Vec<DrcViolation>, String> {
    Ok(graph.run_drc())
}

/// Infers turnout speed ratings using topological heuristics.
#[tauri::command]
fn infer_speeds(mut graph: TrackGraph) -> Result<HashMap<String, SpeedClass>, String> {
    Ok(graph.infer_switch_speeds())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            load_demo_project,
            synthesize_routes,
            run_drc,
            infer_speeds
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
