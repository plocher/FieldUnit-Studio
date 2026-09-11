use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use super::model::{Direction, SpeedClass, SwitchPosition};

/// Unique identifier for a graph node.
pub type NodeId = String;

/// The functional classification of a topology node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeKind {
    Boundary { boundary_id: String, direction: Direction },
    Irj { id: String, circuit_left: String, circuit_right: String },
    SwitchPoints { switch_id: String },
    Bumper { id: String },
    Junction { id: String },
}

/// A node in the track topology graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackNode {
    pub id: NodeId,
    pub kind: NodeKind,
    pub x: f64,
    pub y: f64,
}

/// The track connection type along a directed edge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EdgeKind {
    Tangent { circuit_id: String },
    SwitchNormal { switch_id: String, circuit_id: String },
    SwitchReverse { switch_id: String, circuit_id: String, speed: SpeedClass },
}

/// A directed edge in the track topology graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackEdge {
    pub id: String,
    pub from: NodeId,
    pub to: NodeId,
    pub kind: EdgeKind,
    pub length_feet: f64,
}

impl TrackEdge {
    pub fn circuit_id(&self) -> &str {
        match &self.kind {
            EdgeKind::Tangent { circuit_id } => circuit_id,
            EdgeKind::SwitchNormal { circuit_id, .. } => circuit_id,
            EdgeKind::SwitchReverse { circuit_id, .. } => circuit_id,
        }
    }

    pub fn switch_info(&self) -> Option<(&str, SwitchPosition, SpeedClass)> {
        match &self.kind {
            EdgeKind::Tangent { .. } => None,
            EdgeKind::SwitchNormal { switch_id, .. } => Some((switch_id, SwitchPosition::Normal, SpeedClass::Normal)),
            EdgeKind::SwitchReverse { switch_id, speed, .. } => Some((switch_id, SwitchPosition::Reverse, *speed)),
        }
    }
}

/// Severity classification for Design Rule Checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DrcSeverity {
    Draft,
    Warning,
    Conflict,
}

/// A Design Rule Check diagnostic item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrcViolation {
    pub id: String,
    pub severity: DrcSeverity,
    pub message: String,
    pub node_id: Option<NodeId>,
}

/// The complete directed track graph of a Control Point.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TrackGraph {
    pub nodes: HashMap<NodeId, TrackNode>,
    pub edges: Vec<TrackEdge>,
}

impl TrackGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: TrackNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn add_edge(&mut self, edge: TrackEdge) {
        self.edges.push(edge);
    }

    /// Returns outgoing edges from a given node.
    pub fn outgoing_edges(&self, node_id: &str) -> Vec<&TrackEdge> {
        self.edges.iter().filter(|e| e.from == node_id).collect()
    }

    /// Returns incoming edges to a given node.
    pub fn incoming_edges(&self, node_id: &str) -> Vec<&TrackEdge> {
        self.edges.iter().filter(|e| e.to == node_id).collect()
    }

    /// Infers operational speed ratings for turnouts using topology heuristics.
    pub fn infer_switch_speeds(&mut self) -> HashMap<String, SpeedClass> {
        let mut results = HashMap::new();

        for edge in &self.edges {
            if let EdgeKind::SwitchReverse { switch_id, .. } = &edge.kind {
                let speed = self.evaluate_diverging_speed(&edge.to);
                results.insert(switch_id.clone(), speed);
            }
        }

        // Apply inferred speeds back to the reverse edges
        for edge in &mut self.edges {
            if let EdgeKind::SwitchReverse { switch_id, speed, .. } = &mut edge.kind {
                if let Some(inferred) = results.get(switch_id) {
                    *speed = *inferred;
                }
            }
        }

        results
    }

    /// Evaluates if a downstream path hits a bumper (Slow) or loops back (Medium).
    fn evaluate_diverging_speed(&self, start_node: &str) -> SpeedClass {
        let mut visited = HashSet::new();
        let mut queue = vec![start_node.to_string()];

        while let Some(current) = queue.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }

            if let Some(node) = self.nodes.get(&current) {
                if matches!(node.kind, NodeKind::Bumper { .. }) {
                    return SpeedClass::Slow;
                }
            }

            for edge in self.outgoing_edges(&current) {
                queue.push(edge.to.clone());
            }
        }

        SpeedClass::Medium
    }

    /// Executes Design Rule Checks on the track topology.
    pub fn run_drc(&self) -> Vec<DrcViolation> {
        let mut violations = Vec::new();

        // Rule 1: Check for open unconnected nodes
        for id in self.nodes.keys() {
            let out_count = self.outgoing_edges(id).len();
            let in_count = self.incoming_edges(id).len();

            if out_count == 0 && in_count == 0 {
                violations.push(DrcViolation {
                    id: format!("unconnected-node-{}", id),
                    severity: DrcSeverity::Draft,
                    message: format!("Node '{}' has no track connections.", id),
                    node_id: Some(id.clone()),
                });
            }
        }

        // Rule 2: Ensure switch points have both normal and reverse legs
        let mut switch_legs: HashMap<String, HashSet<SwitchPosition>> = HashMap::new();
        for edge in &self.edges {
            if let Some((sw_id, pos, _)) = edge.switch_info() {
                switch_legs.entry(sw_id.to_string()).or_default().insert(pos);
            }
        }

        for (sw_id, positions) in switch_legs {
            if !positions.contains(&SwitchPosition::Normal) {
                violations.push(DrcViolation {
                    id: format!("missing-normal-leg-{}", sw_id),
                    severity: DrcSeverity::Conflict,
                    message: format!("Switch '{}' is missing a Normal track leg.", sw_id),
                    node_id: None,
                });
            }
            if !positions.contains(&SwitchPosition::Reverse) {
                violations.push(DrcViolation {
                    id: format!("missing-reverse-leg-{}", sw_id),
                    severity: DrcSeverity::Conflict,
                    message: format!("Switch '{}' is missing a Reverse track leg.", sw_id),
                    node_id: None,
                });
            }
        }

        violations
    }
}
