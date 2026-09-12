pub mod core;

use std::collections::HashMap;
use std::sync::Arc;
use core::aar_codec::ControlSnapshot;
use core::corridor_matrix::CorridorMatrix;
use core::graph::{DrcViolation, TrackGraph};
use core::model::SpeedClass;
use core::mqtt_codeline::{CodelineStatus, MqttCodelineManager, MqttConfig};
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

/// Connects to an MQTT broker for Interface "A" codeline communications.
#[tauri::command]
fn codeline_connect(
    state: tauri::State<Arc<MqttCodelineManager>>,
    host: String,
    port: u16,
    layout: String,
) -> Result<(), String> {
    state.connect(MqttConfig {
        host,
        port,
        layout,
        client_id: None,
    })
}

/// Disconnects from the MQTT broker.
#[tauri::command]
fn codeline_disconnect(state: tauri::State<Arc<MqttCodelineManager>>) -> Result<(), String> {
    state.disconnect();
    Ok(())
}

/// Publishes structured AAR control tokens for a station on /layout/<name>/codeline/<cp>/controls.
#[tauri::command]
fn codeline_publish_controls(
    state: tauri::State<Arc<MqttCodelineManager>>,
    cp_name: String,
    switches: Vec<String>,
    signals: Vec<String>,
    snapshot: ControlSnapshot,
) -> Result<(), String> {
    let sw_refs: Vec<&str> = switches.iter().map(|s| s.as_str()).collect();
    let sig_refs: Vec<&str> = signals.iter().map(|s| s.as_str()).collect();
    state.publish_controls(&cp_name, &sw_refs, &sig_refs, &snapshot)
}

/// Publishes raw AAR control tokens string for a station on /layout/<name>/codeline/<cp>/controls.
#[tauri::command]
fn codeline_publish_raw_controls(
    state: tauri::State<Arc<MqttCodelineManager>>,
    cp_name: String,
    tokens: String,
) -> Result<(), String> {
    state.publish_raw_controls(&cp_name, &tokens)
}

/// Role 4: Publishes the authoritative plant JSON specification (retained) to /layout/<name>/codeline/<cp>/json.
#[tauri::command]
fn codeline_publish_plant_json(
    state: tauri::State<Arc<MqttCodelineManager>>,
    cp_name: String,
    plant_json: String,
) -> Result<(), String> {
    state.publish_plant_json(&cp_name, &plant_json)
}

/// Role 4: Retrieves retained plant JSON received from the broker or published locally.
#[tauri::command]
fn codeline_get_plant_json(
    state: tauri::State<Arc<MqttCodelineManager>>,
    cp_name: String,
) -> Result<Option<String>, String> {
    Ok(state.get_plant_json(&cp_name))
}

/// Role 4: Lists all known control point names discovered via /json on the broker.
#[tauri::command]
fn codeline_list_known_plants(
    state: tauri::State<Arc<MqttCodelineManager>>,
) -> Result<Vec<String>, String> {
    Ok(state.list_known_plants())
}

/// Returns the current MQTT connection status.
#[tauri::command]
fn codeline_get_status(
    state: tauri::State<Arc<MqttCodelineManager>>,
) -> Result<CodelineStatus, String> {
    Ok(state.get_status())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let codeline_manager = Arc::new(MqttCodelineManager::new());
    let codeline_clone = Arc::clone(&codeline_manager);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(codeline_manager)
        .setup(move |app| {
            codeline_clone.set_app_handle(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_demo_project,
            synthesize_routes,
            run_drc,
            infer_speeds,
            project_matrix_coordinates,
            matrix_insert_column,
            matrix_rotate_switch,
            matrix_flip_switch,
            codeline_connect,
            codeline_disconnect,
            codeline_publish_controls,
            codeline_publish_raw_controls,
            codeline_publish_plant_json,
            codeline_get_plant_json,
            codeline_list_known_plants,
            codeline_get_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
