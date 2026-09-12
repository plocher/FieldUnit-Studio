use super::corridor_matrix::{
    CorridorMatrix, LinkType, MatrixApplianceType, MatrixCell, MatrixLink,
};
use super::graph::{EdgeKind, NodeKind, TrackEdge, TrackGraph, TrackNode};
use super::model::{
    BoundaryType, ControlPoint, CpBoundary, Direction, Indication, MastType, SignalMast,
    SpeedClass, Switch, SwitchPosition, TrackCircuit,
};
use super::project::ProjectFile;
use super::route_synthesizer::RouteSynthesizer;

#[test]
fn test_heuristic_speed_classification() {
    let mut graph = TrackGraph::new();

    // Node 0: Switch points
    graph.add_node(TrackNode {
        id: "SW1_PTS".to_string(),
        kind: NodeKind::SwitchPoints { switch_id: "SW1".to_string() },
        x: 0.0,
        y: 0.0,
    });

    // Node 1: Normal exit boundary
    graph.add_node(TrackNode {
        id: "MAIN_EXIT".to_string(),
        kind: NodeKind::Boundary { boundary_id: "EAST".to_string(), direction: Direction::Right },
        x: 100.0,
        y: 0.0,
    });

    // Node 2: Reverse bumper (stub lead)
    graph.add_node(TrackNode {
        id: "SPUR_BUMPER".to_string(),
        kind: NodeKind::Bumper { id: "B1".to_string() },
        x: 100.0,
        y: 50.0,
    });

    graph.add_edge(TrackEdge {
        id: "E_NORM".to_string(),
        from: "SW1_PTS".to_string(),
        to: "MAIN_EXIT".to_string(),
        kind: EdgeKind::SwitchNormal { switch_id: "SW1".to_string(), circuit_id: "1T1".to_string() },
        length_feet: 100.0,
    });

    graph.add_edge(TrackEdge {
        id: "E_REV".to_string(),
        from: "SW1_PTS".to_string(),
        to: "SPUR_BUMPER".to_string(),
        kind: EdgeKind::SwitchReverse {
            switch_id: "SW1".to_string(),
            circuit_id: "1T1".to_string(),
            speed: SpeedClass::Medium,
        },
        length_feet: 100.0,
    });

    let inferred = graph.infer_switch_speeds();
    assert_eq!(inferred.get("SW1"), Some(&SpeedClass::Slow), "Reverse lead terminating at bumper must infer Slow speed");
}

#[test]
fn test_route_synthesis_end_of_siding() {
    let mut cp = ControlPoint::new("CP_END_SIDING", "CP End of Siding", "Coast Sub");
    cp.switches.push(Switch::new("SW1", "Switch 1"));
    cp.track_circuits.push(TrackCircuit::new("1T1", "Island 1T1", true));
    cp.track_circuits.push(TrackCircuit::new("1SA", "Approach West", false));
    cp.track_circuits.push(TrackCircuit::new("1NA", "Main Exit East", false));
    cp.track_circuits.push(TrackCircuit::new("2NA", "Siding Exit East", false));

    let mut mast2sab = SignalMast::new("2Sab", "Signal 2Sab", MastType::TwoHead, Direction::Right);
    mast2sab.irj_node_id = Some("IRJ_WEST".to_string());
    cp.signal_masts.push(mast2sab);

    cp.boundaries.push(CpBoundary {
        id: "B_WEST".to_string(),
        name: "West Boundary".to_string(),
        direction: Direction::Left,
        boundary_type: BoundaryType::Abs,
        connected_track_id: Some("1SA".to_string()),
    });
    cp.boundaries.push(CpBoundary {
        id: "B_EAST_MAIN".to_string(),
        name: "East Main Boundary".to_string(),
        direction: Direction::Right,
        boundary_type: BoundaryType::Abs,
        connected_track_id: Some("1NA".to_string()),
    });
    cp.boundaries.push(CpBoundary {
        id: "B_EAST_SIDING".to_string(),
        name: "East Siding Boundary".to_string(),
        direction: Direction::Right,
        boundary_type: BoundaryType::Dark,
        connected_track_id: Some("2NA".to_string()),
    });

    let mut graph = TrackGraph::new();
    graph.add_node(TrackNode {
        id: "IRJ_WEST".to_string(),
        kind: NodeKind::Irj { id: "IRJ_1".to_string(), circuit_left: "1SA".to_string(), circuit_right: "1T1".to_string() },
        x: 0.0,
        y: 0.0,
    });
    graph.add_node(TrackNode {
        id: "SW1_PTS".to_string(),
        kind: NodeKind::SwitchPoints { switch_id: "SW1".to_string() },
        x: 50.0,
        y: 0.0,
    });
    graph.add_node(TrackNode {
        id: "B_EAST_MAIN".to_string(),
        kind: NodeKind::Boundary { boundary_id: "EAST_MAIN".to_string(), direction: Direction::Right },
        x: 150.0,
        y: 0.0,
    });
    graph.add_node(TrackNode {
        id: "B_EAST_SIDING".to_string(),
        kind: NodeKind::Boundary { boundary_id: "EAST_SIDING".to_string(), direction: Direction::Right },
        x: 150.0,
        y: 50.0,
    });

    // Edges
    graph.add_edge(TrackEdge {
        id: "E_APP".to_string(),
        from: "IRJ_WEST".to_string(),
        to: "SW1_PTS".to_string(),
        kind: EdgeKind::Tangent { circuit_id: "1T1".to_string() },
        length_feet: 50.0,
    });
    graph.add_edge(TrackEdge {
        id: "E_MAIN".to_string(),
        from: "SW1_PTS".to_string(),
        to: "B_EAST_MAIN".to_string(),
        kind: EdgeKind::SwitchNormal { switch_id: "SW1".to_string(), circuit_id: "1T1".to_string() },
        length_feet: 100.0,
    });
    graph.add_edge(TrackEdge {
        id: "E_SIDING".to_string(),
        from: "SW1_PTS".to_string(),
        to: "B_EAST_SIDING".to_string(),
        kind: EdgeKind::SwitchReverse {
            switch_id: "SW1".to_string(),
            circuit_id: "1T1".to_string(),
            speed: SpeedClass::Medium,
        },
        length_feet: 100.0,
    });

    let synthesizer = RouteSynthesizer::new(&cp, &graph);
    let routes = synthesizer.synthesize_all_routes();

    assert_eq!(routes.len(), 2, "Expected exactly 2 routes from Signal 2R");

    let main_route = routes.iter().find(|r| r.exit_node_id == "B_EAST_MAIN").expect("Main route must exist");
    assert_eq!(main_route.name, "WEST-to-EAST_MAIN");
    assert_eq!(main_route.aspect_ceiling, Indication::Clear);
    assert_eq!(main_route.switch_alignments.get("SW1"), Some(&SwitchPosition::Normal));
    assert!(main_route.fleeting_capable);

    let siding_route = routes.iter().find(|r| r.exit_node_id == "B_EAST_SIDING").expect("Siding route must exist");
    assert_eq!(siding_route.name, "WEST-to-EAST_SIDING");
    assert_eq!(siding_route.aspect_ceiling, Indication::DivergingClear);
    assert_eq!(siding_route.switch_alignments.get("SW1"), Some(&SwitchPosition::Reverse));
    assert!(siding_route.call_on_capable);

    // Verify mutual conflict
    assert!(main_route.conflicts_with.contains(&siding_route.id));
    assert!(siding_route.conflicts_with.contains(&main_route.id));
}

#[test]
fn test_concurrent_parallel_routes() {
    let mut cp = ControlPoint::new("CP_DOUBLE_TRACK", "CP Double Track", "Coast Sub");

    let mut mast2r = SignalMast::new("2R", "Signal 2R", MastType::OneHead, Direction::Right);
    mast2r.irj_node_id = Some("IRJ_MT1".to_string());
    cp.signal_masts.push(mast2r);

    let mut mast4r = SignalMast::new("4R", "Signal 4R", MastType::OneHead, Direction::Right);
    mast4r.irj_node_id = Some("IRJ_MT2".to_string());
    cp.signal_masts.push(mast4r);

    let mut graph = TrackGraph::new();
    graph.add_node(TrackNode {
        id: "IRJ_MT1".to_string(),
        kind: NodeKind::Irj { id: "IRJ_1".to_string(), circuit_left: "1SA".to_string(), circuit_right: "1T1".to_string() },
        x: 0.0,
        y: 0.0,
    });
    graph.add_node(TrackNode {
        id: "EXIT_MT1".to_string(),
        kind: NodeKind::Boundary { boundary_id: "EAST_MT1".to_string(), direction: Direction::Right },
        x: 100.0,
        y: 0.0,
    });
    graph.add_node(TrackNode {
        id: "IRJ_MT2".to_string(),
        kind: NodeKind::Irj { id: "IRJ_2".to_string(), circuit_left: "2SA".to_string(), circuit_right: "2T1".to_string() },
        x: 0.0,
        y: 40.0,
    });
    graph.add_node(TrackNode {
        id: "EXIT_MT2".to_string(),
        kind: NodeKind::Boundary { boundary_id: "EAST_MT2".to_string(), direction: Direction::Right },
        x: 100.0,
        y: 40.0,
    });

    graph.add_edge(TrackEdge {
        id: "E_MT1".to_string(),
        from: "IRJ_MT1".to_string(),
        to: "EXIT_MT1".to_string(),
        kind: EdgeKind::Tangent { circuit_id: "1T1".to_string() },
        length_feet: 100.0,
    });
    graph.add_edge(TrackEdge {
        id: "E_MT2".to_string(),
        from: "IRJ_MT2".to_string(),
        to: "EXIT_MT2".to_string(),
        kind: EdgeKind::Tangent { circuit_id: "2T1".to_string() },
        length_feet: 100.0,
    });

    let synthesizer = RouteSynthesizer::new(&cp, &graph);
    let routes = synthesizer.synthesize_all_routes();

    assert_eq!(routes.len(), 2);
    let r1 = &routes[0];
    let r2 = &routes[1];

    assert!(r1.concurrent_with.contains(&r2.id), "Parallel independent routes must be identified as concurrent");
    assert!(r2.concurrent_with.contains(&r1.id));
    assert!(r1.conflicts_with.is_empty());
}

#[test]
fn test_drc_checks() {
    let mut graph = TrackGraph::new();
    graph.add_node(TrackNode {
        id: "DANGLING_NODE".to_string(),
        kind: NodeKind::Junction { id: "J1".to_string() },
        x: 0.0,
        y: 0.0,
    });

    let violations = graph.run_drc();
    assert!(violations.iter().any(|v| v.id == "unconnected-node-DANGLING_NODE"));
}

#[test]
fn test_project_serialization_round_trip() {
    let mut project = ProjectFile::new("Santa Cruz Branch", "Coast Division");
    let mut cp = ControlPoint::new("CP_WEST", "CP West", "Coast Division");
    cp.switches.push(Switch::new("SW1", "Switch 1"));
    cp.track_circuits.push(TrackCircuit::new("1T1", "Island 1T1", true));
    project.control_points.push(cp);

    let json = project.to_json().expect("Serialization to JSON must succeed");
    let restored = ProjectFile::from_json(&json).expect("Deserialization from JSON must succeed");

    assert_eq!(project, restored, "Project must round-trip through JSON with complete fidelity");
}

#[test]
fn test_corridor_matrix_slot_expansion() {
    let mut matrix = CorridorMatrix::new();

    // Mainline cells on Level 0
    matrix.add_cell(MatrixCell {
        node_id: "WEST_MAIN".to_string(),
        slot: 0,
        level: 0,
        appliance: MatrixApplianceType::Tangent,
    });
    matrix.add_cell(MatrixCell {
        node_id: "SW1_PTS".to_string(),
        slot: 1,
        level: 0,
        appliance: MatrixApplianceType::SwitchPoints { switch_id: "1".to_string(), facing_east: true, diverge_down: true },
    });
    matrix.add_cell(MatrixCell {
        node_id: "EAST_MAIN".to_string(),
        slot: 2,
        level: 0,
        appliance: MatrixApplianceType::Tangent,
    });

    // Siding cells on Level 1
    matrix.add_cell(MatrixCell {
        node_id: "WEST_SIDING".to_string(),
        slot: 0,
        level: 1,
        appliance: MatrixApplianceType::Tangent,
    });
    matrix.add_cell(MatrixCell {
        node_id: "EAST_SIDING".to_string(),
        slot: 2,
        level: 1,
        appliance: MatrixApplianceType::Tangent,
    });

    // 45° Diverge link from (Slot 1, Level 0) to (Slot 2, Level 1)
    matrix.add_link(MatrixLink {
        id: "L_DIV".to_string(),
        from_node: "SW1_PTS".to_string(),
        to_node: "EAST_SIDING".to_string(),
        from_slot: 1,
        from_level: 0,
        to_slot: 2,
        to_level: 1,
        link_type: LinkType::Diagonal45,
        circuit_id: "1T".to_string(),
    });

    assert!(matrix.validate_geometry().is_empty(), "Initial geometry must be valid");

    // Insert a new station column at slot 2 (expanding the corridor matrix)
    matrix.insert_station_column(2);

    let cell_east_main = matrix.cells.iter().find(|c| c.node_id == "EAST_MAIN").unwrap();
    let cell_east_siding = matrix.cells.iter().find(|c| c.node_id == "EAST_SIDING").unwrap();
    assert_eq!(cell_east_main.slot, 3, "All downstream nodes on Level 0 must shift to slot 3");
    assert_eq!(cell_east_siding.slot, 3, "All downstream nodes on Level 1 must shift to slot 3 in unison");

    let link_div = matrix.links.iter().find(|l| l.id == "L_DIV").unwrap();
    assert_eq!(link_div.to_slot, 3, "Link target must shift in unison");
}

#[test]
fn test_corridor_matrix_rotate_and_flip() {
    let mut matrix = CorridorMatrix::new();
    matrix.add_cell(MatrixCell {
        node_id: "SW1_PTS".to_string(),
        slot: 2,
        level: 0,
        appliance: MatrixApplianceType::SwitchPoints {
            switch_id: "1".to_string(),
            facing_east: true,
            diverge_down: true,
        },
    });
    matrix.add_link(MatrixLink {
        id: "L_REV".to_string(),
        from_node: "SW1_PTS".to_string(),
        to_node: "SIDING_JOINT".to_string(),
        from_slot: 2,
        from_level: 0,
        to_slot: 3,
        to_level: 1,
        link_type: LinkType::Diagonal45,
        circuit_id: "1T".to_string(),
    });

    // Flip switch diverge side (Diverge Down -> Diverge Up)
    assert!(matrix.flip_switch("1"));
    let link_after_flip = matrix.links.iter().find(|l| l.id == "L_REV").unwrap();
    assert_eq!(link_after_flip.to_level, -1, "Flipping must redirect reverse branch to Level -1");

    // Rotate switch facing direction (Facing East -> Facing West / Trailing)
    assert!(matrix.rotate_switch("1"));
    let link_after_rot = matrix.links.iter().find(|l| l.id == "L_REV").unwrap();
    assert_eq!(link_after_rot.to_slot, 1, "Rotating must redirect reverse branch Westward to Slot 1");
}

#[test]
fn test_corridor_matrix_coordinate_projection() {
    let mut matrix = CorridorMatrix::new();
    matrix.origin_x_px = 100.0;
    matrix.origin_y_px = 200.0;
    matrix.col_spacing_px = 150.0;
    matrix.row_spacing_px = 80.0;

    matrix.add_cell(MatrixCell {
        node_id: "NODE_A".to_string(),
        slot: 0,
        level: 0,
        appliance: MatrixApplianceType::Tangent,
    });
    matrix.add_cell(MatrixCell {
        node_id: "NODE_B".to_string(),
        slot: 2,
        level: 1,
        appliance: MatrixApplianceType::Tangent,
    });

    let coords = matrix.project_coordinates();
    assert_eq!(coords.get("NODE_A"), Some(&(100.0, 200.0)));
    assert_eq!(coords.get("NODE_B"), Some(&(400.0, 280.0)));
}
