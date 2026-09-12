pub mod core;

use std::collections::HashMap;
use core::corridor_matrix::CorridorMatrix;
use core::graph::{DrcViolation, TrackGraph};
use core::model::SpeedClass;
use core::project::ProjectFile;
use core::route_synthesizer::{RouteSynthesizer, SynthesizedRoute};

/// Loads the verified CP Corporal (Southern Pacific Coast Line MP 83) baseline project directly from plant JSON.
#[tauri::command]
fn load_demo_project() -> Result<ProjectFile, String> {
    const CORPORAL_JSON: &str = include_str!("../../docs/CP_Corporal.json");
    ProjectFile::from_plant_json(CORPORAL_JSON).map_err(|e| e.to_string())
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

/// Computes coordinate projections from a CorridorMatrix.
#[tauri::command]
fn project_matrix_coordinates(matrix: CorridorMatrix) -> Result<HashMap<String, (f64, f64)>, String> {
    Ok(matrix.project_coordinates())
}

/// Inserts a new station column into a CorridorMatrix.
#[tauri::command]
fn matrix_insert_column(mut matrix: CorridorMatrix, slot: usize) -> Result<CorridorMatrix, String> {
    matrix.insert_station_column(slot);
    Ok(matrix)
}

/// Rotates a switch in a CorridorMatrix (Facing East <-> Facing West / Trailing).
#[tauri::command]
fn matrix_rotate_switch(mut matrix: CorridorMatrix, switch_id: String) -> Result<CorridorMatrix, String> {
    matrix.rotate_switch(&switch_id);
    Ok(matrix)
}

/// Flips a switch diverge side in a CorridorMatrix (Diverge Up <-> Diverge Down).
#[tauri::command]
fn matrix_flip_switch(mut matrix: CorridorMatrix, switch_id: String) -> Result<CorridorMatrix, String> {
    matrix.flip_switch(&switch_id);
    Ok(matrix)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            load_demo_project,
            synthesize_routes,
            run_drc,
            infer_speeds,
            project_matrix_coordinates,
            matrix_insert_column,
            matrix_rotate_switch,
            matrix_flip_switch
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
