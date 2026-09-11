use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use super::graph::TrackGraph;
use super::model::ControlPoint;
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
}
