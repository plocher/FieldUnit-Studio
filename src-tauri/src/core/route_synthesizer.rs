use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use super::graph::{NodeKind, TrackGraph};
use super::model::{ControlPoint, Direction, Indication, SignalMast, SpeedClass, SwitchPosition};

/// A synthesized route through an interlocking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynthesizedRoute {
    pub id: String,
    pub name: String,
    pub entrance_signal_id: String,
    pub exit_node_id: String,
    pub direction: Direction,
    pub switch_alignments: HashMap<String, SwitchPosition>,
    pub clears_circuits: Vec<String>,
    pub aspect_ceiling: Indication,
    pub fleeting_capable: bool,
    pub call_on_capable: bool,
    pub conflicts_with: Vec<String>,
    pub concurrent_with: Vec<String>,
}

/// Synthesizes routes, conflicts, and concurrency across a track plant.
pub struct RouteSynthesizer<'a> {
    pub cp: &'a ControlPoint,
    pub graph: &'a TrackGraph,
}

impl<'a> RouteSynthesizer<'a> {
    pub fn new(cp: &'a ControlPoint, graph: &'a TrackGraph) -> Self {
        Self { cp, graph }
    }

    /// Discovers all valid interlocking routes through the track graph.
    pub fn synthesize_all_routes(&self) -> Vec<SynthesizedRoute> {
        let mut routes = Vec::new();

        for mast in &self.cp.signal_masts {
            if let Some(irj_node) = &mast.irj_node_id {
                let discovered = self.find_routes_from_signal(mast, irj_node);
                routes.extend(discovered);
            }
        }

        // Calculate pairwise conflicts and concurrency
        self.compute_relationships(&mut routes);
        routes
    }

    fn find_routes_from_signal(&self, mast: &SignalMast, start_node: &str) -> Vec<SynthesizedRoute> {
        let mut routes = Vec::new();

        // Path search state: (current_node, switch_alignments, circuits, speeds, visited_nodes)
        let initial_state = (
            start_node.to_string(),
            HashMap::new(),
            Vec::new(),
            Vec::new(),
            HashSet::new(),
        );

        let mut queue = vec![initial_state];

        while let Some((curr, alignments, circuits, speeds, mut visited)) = queue.pop() {
            if !visited.insert(curr.clone()) {
                continue;
            }

            // Check if current node is a termination point (Boundary or Bumper) and not the starting node
            if curr != start_node {
                if let Some(node) = self.graph.nodes.get(&curr) {
                    let is_exit = matches!(node.kind, NodeKind::Boundary { .. } | NodeKind::Bumper { .. });
                    if is_exit {
                        let route = self.build_route(mast, &curr, alignments.clone(), circuits.clone(), speeds.clone());
                        routes.push(route);
                        continue;
                    }
                }
            }

            for edge in self.graph.outgoing_edges(&curr) {
                let mut next_alignments = alignments.clone();
                let mut next_circuits = circuits.clone();
                let mut next_speeds = speeds.clone();

                let circuit_id = edge.circuit_id().to_string();
                if !next_circuits.contains(&circuit_id) {
                    next_circuits.push(circuit_id);
                }

                if let Some((sw_id, pos, speed)) = edge.switch_info() {
                    // If switch already aligned to opposite position, this branch is invalid
                    if let Some(existing) = next_alignments.get(sw_id) {
                        if *existing != pos {
                            continue;
                        }
                    }
                    next_alignments.insert(sw_id.to_string(), pos);
                    next_speeds.push(speed);
                }

                queue.push((edge.to.clone(), next_alignments, next_circuits, next_speeds, visited.clone()));
            }
        }

        routes
    }

    fn build_route(
        &self,
        mast: &SignalMast,
        exit_node: &str,
        alignments: HashMap<String, SwitchPosition>,
        circuits: Vec<String>,
        speeds: Vec<SpeedClass>,
    ) -> SynthesizedRoute {
        // Derive aspect ceiling using the Rule of the Minimum
        let min_speed = speeds.iter().min().copied().unwrap_or(SpeedClass::Normal);
        let ceiling = match min_speed {
            SpeedClass::Normal => Indication::Clear,
            SpeedClass::Limited => Indication::LimitedClear,
            SpeedClass::Medium => Indication::DivergingClear,
            SpeedClass::Slow => Indication::SlowClear,
        };

        let is_straight = alignments.values().all(|p| *p == SwitchPosition::Normal);
        let route_name = format!("{}-to-{}", mast.name, exit_node);

        SynthesizedRoute {
            id: format!("RT_{}_{}", mast.id, exit_node),
            name: route_name,
            entrance_signal_id: mast.id.clone(),
            exit_node_id: exit_node.to_string(),
            direction: mast.direction,
            switch_alignments: alignments,
            clears_circuits: circuits,
            aspect_ceiling: ceiling,
            fleeting_capable: is_straight,
            call_on_capable: !is_straight,
            conflicts_with: Vec::new(),
            concurrent_with: Vec::new(),
        }
    }

    fn compute_relationships(&self, routes: &mut [SynthesizedRoute]) {
        let count = routes.len();
        for i in 0..count {
            for j in (i + 1)..count {
                let is_conflict = self.are_conflicting(&routes[i], &routes[j]);
                let id_i = routes[i].id.clone();
                let id_j = routes[j].id.clone();

                if is_conflict {
                    routes[i].conflicts_with.push(id_j.clone());
                    routes[j].conflicts_with.push(id_i);
                } else {
                    routes[i].concurrent_with.push(id_j.clone());
                    routes[j].concurrent_with.push(id_i);
                }
            }
        }
    }

    fn are_conflicting(&self, r1: &SynthesizedRoute, r2: &SynthesizedRoute) -> bool {
        // Conflict 1: Shared track circuits
        let set1: HashSet<_> = r1.clears_circuits.iter().collect();
        let set2: HashSet<_> = r2.clears_circuits.iter().collect();
        if !set1.is_disjoint(&set2) {
            return true;
        }

        // Conflict 2: Contradictory switch alignments
        for (sw, pos1) in &r1.switch_alignments {
            if let Some(pos2) = r2.switch_alignments.get(sw) {
                if pos1 != pos2 {
                    return true;
                }
            }
        }

        false
    }
}
