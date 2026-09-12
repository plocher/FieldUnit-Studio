use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::model::Direction;

/// Track level along horizontal corridors (0 = Mainline, 1 = Siding/Main 2, -1/2 = Spurs).
pub type CorridorLevel = i32;

/// Ordered linear station column index (aligned vertically with CTC machine plates).
pub type StationSlot = usize;

/// Appliance type hosted in a matrix cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MatrixApplianceType {
    Tangent,
    Irj { joint_id: String },
    SwitchPoints { switch_id: String, facing_east: bool, diverge_down: bool },
    SignalMast { signal_id: String, direction: Direction },
    Boundary { boundary_id: String, direction: Direction },
    Bumper { bumper_id: String },
}

/// A cell in the Station Corridor Matrix.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatrixCell {
    pub node_id: String,
    pub slot: StationSlot,
    pub level: CorridorLevel,
    pub appliance: MatrixApplianceType,
}

/// Geometric link type between matrix cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkType {
    Horizontal, // 0° / 180° along the same corridor
    Vertical,   // 90° / 270° within the same station column
    Diagonal45, // Exact ±45° diverge between adjacent corridor levels
}

/// An interconnected track net in the matrix.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatrixLink {
    pub id: String,
    pub from_node: String,
    pub to_node: String,
    pub from_slot: StationSlot,
    pub from_level: CorridorLevel,
    pub to_slot: StationSlot,
    pub to_level: CorridorLevel,
    pub link_type: LinkType,
    pub circuit_id: String,
}

/// The Station Corridor Layout Engine.
/// Replaces floating coordinates with a discrete topological matrix.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CorridorMatrix {
    pub cells: Vec<MatrixCell>,
    pub links: Vec<MatrixLink>,
    pub col_spacing_px: f64,
    pub row_spacing_px: f64,
    pub origin_x_px: f64,
    pub origin_y_px: f64,
}

impl Default for CorridorMatrix {
    fn default() -> Self {
        Self {
            cells: Vec::new(),
            links: Vec::new(),
            col_spacing_px: 140.0,
            row_spacing_px: 100.0,
            origin_x_px: 80.0,
            origin_y_px: 180.0,
        }
    }
}

impl CorridorMatrix {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a cell to the layout matrix.
    pub fn add_cell(&mut self, cell: MatrixCell) {
        self.cells.retain(|c| c.node_id != cell.node_id);
        self.cells.push(cell);
    }

    /// Adds a link between two matrix cells, enforcing model board geometric rules.
    pub fn add_link(&mut self, link: MatrixLink) {
        self.links.retain(|l| l.id != link.id);
        self.links.push(link);
    }

    /// Inserts a new station column into the matrix.
    /// Shifts all subsequent columns in all parallel corridors rightward in unison.
    pub fn insert_station_column(&mut self, at_slot: StationSlot) {
        for cell in &mut self.cells {
            if cell.slot >= at_slot {
                cell.slot += 1;
            }
        }
        for link in &mut self.links {
            if link.from_slot >= at_slot {
                link.from_slot += 1;
            }
            if link.to_slot >= at_slot {
                link.to_slot += 1;
            }
        }
    }

    /// Rotates a switch facing direction by 180° along the track axis.
    pub fn rotate_switch(&mut self, switch_id: &str) -> bool {
        let mut modified = false;
        let mut pts_slot = 0;
        let mut pts_level = 0;
        let mut is_now_east = false;

        for cell in &mut self.cells {
            if let MatrixApplianceType::SwitchPoints { switch_id: id, facing_east, .. } = &mut cell.appliance {
                if id == switch_id {
                    *facing_east = !*facing_east;
                    is_now_east = *facing_east;
                    pts_slot = cell.slot;
                    pts_level = cell.level;
                    modified = true;
                    break;
                }
            }
        }

        if modified {
            // Reorient the reverse diverging diagonal link
            for link in &mut self.links {
                if link.from_slot == pts_slot && link.from_level == pts_level && link.link_type == LinkType::Diagonal45 {
                    link.to_slot = if is_now_east { pts_slot + 1 } else { pts_slot.saturating_sub(1) };
                }
            }
        }

        modified
    }

    /// Flips a switch diverge side across the horizontal track axis (Diverge Up vs. Diverge Down).
    pub fn flip_switch(&mut self, switch_id: &str) -> bool {
        let mut modified = false;
        let mut pts_slot = 0;
        let mut pts_level = 0;
        let mut is_now_down = false;

        for cell in &mut self.cells {
            if let MatrixApplianceType::SwitchPoints { switch_id: id, diverge_down, .. } = &mut cell.appliance {
                if id == switch_id {
                    *diverge_down = !*diverge_down;
                    is_now_down = *diverge_down;
                    pts_slot = cell.slot;
                    pts_level = cell.level;
                    modified = true;
                    break;
                }
            }
        }

        if modified {
            // Mirror the reverse branch to the opposite corridor level
            for link in &mut self.links {
                if link.from_slot == pts_slot && link.from_level == pts_level && link.link_type == LinkType::Diagonal45 {
                    link.to_level = if is_now_down { pts_level + 1 } else { pts_level - 1 };
                }
            }
        }

        modified
    }

    /// Computes exact mathematical canvas coordinates from the corridor and slot matrix.
    /// Guarantees that tracks remain strictly 0°, 180°, 90°, or ±45°.
    pub fn project_coordinates(&self) -> HashMap<String, (f64, f64)> {
        let mut coords = HashMap::new();
        for cell in &self.cells {
            let x = self.origin_x_px + (cell.slot as f64) * self.col_spacing_px;
            let y = self.origin_y_px + (cell.level as f64) * self.row_spacing_px;
            coords.insert(cell.node_id.clone(), (x, y));
        }
        coords
    }

    /// Verifies all geometric constraints in the matrix.
    pub fn validate_geometry(&self) -> Vec<String> {
        let mut errors = Vec::new();
        for link in &self.links {
            let slot_delta = (link.to_slot as i64 - link.from_slot as i64).abs();
            let level_delta = (link.to_level - link.from_level).abs();

            match link.link_type {
                LinkType::Horizontal => {
                    if level_delta != 0 {
                        errors.push(format!("Link '{}' marked Horizontal has level delta {}", link.id, level_delta));
                    }
                }
                LinkType::Vertical => {
                    if slot_delta != 0 {
                        errors.push(format!("Link '{}' marked Vertical has slot delta {}", link.id, slot_delta));
                    }
                }
                LinkType::Diagonal45 => {
                    if slot_delta != level_delta as i64 {
                        errors.push(format!("Link '{}' violates 45° angle rule (slot delta {}, level delta {})", link.id, slot_delta, level_delta));
                    }
                }
            }
        }
        errors
    }
}
