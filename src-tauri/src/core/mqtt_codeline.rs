use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use rumqttc::{Client, Event, MqttOptions, Packet, QoS};
use tauri::{AppHandle, Emitter};

use super::aar_codec::{AarCodec, ControlSnapshot, IndicationVector};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CodelineStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttConfig {
    pub host: String,
    pub port: u16,
    pub layout: String,
    pub client_id: Option<String>,
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 1883,
            layout: "spcoast".to_string(),
            client_id: None,
        }
    }
}

/// Incoming indication event sent to Svelte frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicationEvent {
    pub cp_name: String,
    pub vector: IndicationVector,
}

pub struct MqttCodelineManager {
    client: Arc<Mutex<Option<Client>>>,
    config: Arc<Mutex<MqttConfig>>,
    status: Arc<Mutex<CodelineStatus>>,
    known_plants: Arc<Mutex<HashMap<String, String>>>,
    app_handle: Arc<Mutex<Option<AppHandle>>>,
}

impl MqttCodelineManager {
    pub fn new() -> Self {
        Self {
            client: Arc::new(Mutex::new(None)),
            config: Arc::new(Mutex::new(MqttConfig::default())),
            status: Arc::new(Mutex::new(CodelineStatus::Disconnected)),
            known_plants: Arc::new(Mutex::new(HashMap::new())),
            app_handle: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_app_handle(&self, handle: AppHandle) {
        let mut app = self.app_handle.lock().unwrap();
        *app = Some(handle);
    }

    pub fn get_status(&self) -> CodelineStatus {
        self.status.lock().unwrap().clone()
    }

    pub fn get_config(&self) -> MqttConfig {
        self.config.lock().unwrap().clone()
    }

    /// Connects to the MQTT broker and starts background event polling.
    pub fn connect(&self, config: MqttConfig) -> Result<(), String> {
        // Disconnect existing if active
        self.disconnect();

        {
            let mut cfg = self.config.lock().unwrap();
            *cfg = config.clone();
        }

        let client_id = config
            .client_id
            .clone()
            .unwrap_or_else(|| format!("studio-ctc-{}", std::process::id()));

        let mut mqttoptions = MqttOptions::new(client_id, &config.host, config.port);
        mqttoptions.set_keep_alive(Duration::from_secs(5));
        mqttoptions.set_clean_session(true);

        let (client, mut connection) = Client::new(mqttoptions, 64);

        // Subscribe to layout codeline topics
        let indications_topic = format!("/layout/{}/codeline/+/indications", config.layout);
        let json_topic = format!("/layout/{}/codeline/+/json", config.layout);
        let telemetry_topic = format!("/layout/{}/codeline/+/telemetry", config.layout);

        client
            .subscribe(&indications_topic, QoS::AtLeastOnce)
            .map_err(|e| e.to_string())?;
        client
            .subscribe(&json_topic, QoS::AtLeastOnce)
            .map_err(|e| e.to_string())?;
        client
            .subscribe(&telemetry_topic, QoS::AtLeastOnce)
            .map_err(|e| e.to_string())?;

        {
            let mut cl = self.client.lock().unwrap();
            *cl = Some(client);
            let mut st = self.status.lock().unwrap();
            *st = CodelineStatus::Connected;
        }

        self.emit_status(CodelineStatus::Connected);

        let status_clone = Arc::clone(&self.status);
        let app_handle_clone = Arc::clone(&self.app_handle);
        let known_plants_clone = Arc::clone(&self.known_plants);
        let layout_name = config.layout.clone();

        // Background receiver thread
        thread::spawn(move || {
            for notification in connection.iter() {
                match notification {
                    Ok(Event::Incoming(Packet::Publish(publish))) => {
                        let topic = publish.topic;
                        let payload_str = String::from_utf8_lossy(&publish.payload).to_string();

                        // Match: /layout/<layout>/codeline/<cp>/indications
                        let prefix = format!("/layout/{}/codeline/", layout_name);
                        if let Some(rest) = topic.strip_prefix(&prefix) {
                            let parts: Vec<&str> = rest.split('/').collect();
                            if parts.len() == 2 {
                                let cp_name = parts[0].to_string();
                                let channel = parts[1];

                                if channel == "indications" {
                                    let vector = AarCodec::parse_indications(&payload_str);
                                    let event = IndicationEvent {
                                        cp_name: cp_name.clone(),
                                        vector,
                                    };
                                    if let Some(app) = app_handle_clone.lock().unwrap().as_ref() {
                                        let _ = app.emit("codeline:indication", &event);
                                    }
                                } else if channel == "json" {
                                    known_plants_clone
                                        .lock()
                                        .unwrap()
                                        .insert(cp_name.clone(), payload_str.clone());
                                    if let Some(app) = app_handle_clone.lock().unwrap().as_ref() {
                                        let _ = app.emit("codeline:json", serde_json::json!({
                                            "cp_name": cp_name,
                                            "json": payload_str
                                        }));
                                    }
                                } else if channel == "telemetry" {
                                    if let Some(app) = app_handle_clone.lock().unwrap().as_ref() {
                                        let _ = app.emit("codeline:telemetry", serde_json::json!({
                                            "cp_name": cp_name,
                                            "telemetry": payload_str
                                        }));
                                    }
                                }
                            }
                        }
                    }
                    Ok(Event::Incoming(Packet::ConnAck(_))) => {
                        let mut st = status_clone.lock().unwrap();
                        *st = CodelineStatus::Connected;
                    }
                    Err(e) => {
                        let mut st = status_clone.lock().unwrap();
                        *st = CodelineStatus::Error(e.to_string());
                        // Connection broke or connection closed
                        break;
                    }
                    _ => {}
                }
            }

            let mut st = status_clone.lock().unwrap();
            *st = CodelineStatus::Disconnected;
        });

        Ok(())
    }

    pub fn disconnect(&self) {
        let mut cl = self.client.lock().unwrap();
        if let Some(client) = cl.take() {
            let _ = client.disconnect();
        }
        let mut st = self.status.lock().unwrap();
        *st = CodelineStatus::Disconnected;
        self.emit_status(CodelineStatus::Disconnected);
    }

    /// Publishes a transient control snapshot to /layout/<name>/codeline/<cp>/controls (QoS 1, retain: false).
    pub fn publish_controls(
        &self,
        cp_name: &str,
        ordered_switches: &[&str],
        ordered_signals: &[&str],
        snapshot: &ControlSnapshot,
    ) -> Result<(), String> {
        let tokens = AarCodec::format_controls(ordered_switches, ordered_signals, snapshot);
        self.publish_raw_controls(cp_name, &tokens)
    }

    /// Publishes raw AAR control tokens string to /layout/<name>/codeline/<cp>/controls.
    pub fn publish_raw_controls(&self, cp_name: &str, tokens: &str) -> Result<(), String> {
        let layout = self.config.lock().unwrap().layout.clone();
        let topic = format!("/layout/{}/codeline/{}/controls", layout, cp_name);

        let mut cl = self.client.lock().unwrap();
        if let Some(client) = cl.as_mut() {
            client
                .publish(topic, QoS::AtLeastOnce, false, tokens.as_bytes())
                .map_err(|e| e.to_string())
        } else {
            Err("MQTT client not connected".to_string())
        }
    }

    /// Role 4: Publishes the authoritative plant JSON specification (retained = true).
    pub fn publish_plant_json(&self, cp_name: &str, plant_json: &str) -> Result<(), String> {
        self.known_plants
            .lock()
            .unwrap()
            .insert(cp_name.to_string(), plant_json.to_string());

        let layout = self.config.lock().unwrap().layout.clone();
        let topic = format!("/layout/{}/codeline/{}/json", layout, cp_name);

        let mut cl = self.client.lock().unwrap();
        if let Some(client) = cl.as_mut() {
            client
                .publish(topic, QoS::AtLeastOnce, true, plant_json.as_bytes())
                .map_err(|e| e.to_string())
        } else {
            Err("MQTT client not connected".to_string())
        }
    }

    /// Retrieves cached plant JSON received from the broker or published locally.
    pub fn get_plant_json(&self, cp_name: &str) -> Option<String> {
        self.known_plants.lock().unwrap().get(cp_name).cloned()
    }

    /// Returns list of all known control point names discovered via /json.
    pub fn list_known_plants(&self) -> Vec<String> {
        let plants = self.known_plants.lock().unwrap();
        let mut list: Vec<String> = plants.keys().cloned().collect();
        list.sort();
        list
    }

    fn emit_status(&self, status: CodelineStatus) {
        if let Some(app) = self.app_handle.lock().unwrap().as_ref() {
            let _ = app.emit("codeline:status", &status);
        }
    }
}

impl Default for MqttCodelineManager {
    fn default() -> Self {
        Self::new()
    }
}
