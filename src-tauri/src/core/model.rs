use serde::{Deserialize, Serialize};

/// Operational speed classes for turnouts and routes.
/// Decoupled from physical scale frog numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SpeedClass {
    Slow,
    Medium,
    Limited,
    Normal,
}

impl Default for SpeedClass {
    fn default() -> Self {
        SpeedClass::Medium
    }
}

/// Standard Association of American Railroads (AAR) indications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Indication {
    Stop,
    Restricting,
    SlowClear,
    DivergingClear,
    LimitedClear,
    Approach,
    Clear,
}

/// Timetable traffic flow direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    Left,  // Westward / Northward
    Right, // Eastward / Southward
}

/// Commanded or reported switch point position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SwitchPosition {
    Normal,
    Reverse,
}

/// A track switch appliance.
/// Manages motor outputs, point detection inputs, and operational speed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Switch {
    pub id: String,
    pub name: String,
    pub speed: SpeedClass,
    pub motor_pin: Option<u8>,
    pub normal_sense_pin: Option<u8>,
    pub reverse_sense_pin: Option<u8>,
    pub island_circuit_id: Option<String>,
}

impl Switch {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            speed: SpeedClass::Medium,
            motor_pin: None,
            normal_sense_pin: None,
            reverse_sense_pin: None,
            island_circuit_id: None,
        }
    }
}

/// A paired crossover appliance.
/// Drives and proves two physical switch machines in unison.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Crossover {
    pub id: String,
    pub name: String,
    pub speed: SpeedClass,
    pub switch_a_id: String,
    pub switch_b_id: String,
}

/// A track detection circuit.
/// Models current block occupancy, optical fouling sensors, and dropout delay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackCircuit {
    pub id: String,
    pub name: String,
    pub is_island: bool,
    pub dropout_delay_ms: u32,
    pub sensor_pin: Option<u8>,
    pub optical_pin: Option<u8>,
}

impl TrackCircuit {
    pub fn new(id: impl Into<String>, name: impl Into<String>, is_island: bool) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            is_island,
            dropout_delay_ms: if is_island { 2000 } else { 100 },
            sensor_pin: None,
            optical_pin: None,
        }
    }
}

/// Physical signal mast configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MastType {
    OneHead,
    TwoHead,
    ThreeHead,
    Dwarf,
}

/// A wayside signal mast appliance.
/// Governs movement into the interlocking at an insulated rail joint boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignalMast {
    pub id: String,
    pub name: String,
    pub mast_type: MastType,
    pub direction: Direction,
    pub governing_lever: Option<String>,
    pub irj_node_id: Option<String>,
}

impl SignalMast {
    pub fn new(id: impl Into<String>, name: impl Into<String>, mast_type: MastType, direction: Direction) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            mast_type,
            direction,
            governing_lever: None,
            irj_node_id: None,
        }
    }
}

/// External signaling territory connection type at a CP boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoundaryType {
    Abs,
    Apb,
    Ctc,
    Dark,
}

/// A Control Point boundary node representing interlocking limits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CpBoundary {
    pub id: String,
    pub name: String,
    pub direction: Direction,
    pub boundary_type: BoundaryType,
    pub connected_track_id: Option<String>,
}

/// A Control Point model containing all appliances and configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlPoint {
    pub id: String,
    pub name: String,
    pub subdivision: String,
    pub milepost: Option<f64>,
    pub switches: Vec<Switch>,
    pub crossovers: Vec<Crossover>,
    pub track_circuits: Vec<TrackCircuit>,
    pub signal_masts: Vec<SignalMast>,
    pub boundaries: Vec<CpBoundary>,
}

impl ControlPoint {
    pub fn new(id: impl Into<String>, name: impl Into<String>, subdivision: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            subdivision: subdivision.into(),
            milepost: None,
            switches: Vec::new(),
            crossovers: Vec::new(),
            track_circuits: Vec::new(),
            signal_masts: Vec::new(),
            boundaries: Vec::new(),
        }
    }
}
