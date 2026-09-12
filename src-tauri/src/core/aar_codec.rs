use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parsed indication vector received from a control point across Interface "A".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct IndicationVector {
    pub raw: String,
    /// Switch positions: "Normal", "Reverse", "Moving" (out of correspondence)
    pub switches: HashMap<String, String>,
    /// Track circuit occupancies: true = occupied, false = vacant
    pub tracks: HashMap<String, bool>,
    /// Signal authorities: "Stop", "Left", "Right"
    pub signals: HashMap<String, String>,
    /// Active approach time locks: true = time locking countdown active
    pub time_locks: HashMap<String, bool>,
    /// Maintainer call indicator
    pub maintainer_call: bool,
}

/// Control snapshot transmitted from the cTc console to a control point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ControlSnapshot {
    pub cp_name: String,
    /// Demanded switch positions: "Normal" | "Reverse"
    pub switches: HashMap<String, String>,
    /// Demanded signal authorities: "Left" | "Stop" | "Right"
    pub signals: HashMap<String, String>,
    pub maintainer_call: Option<bool>,
}

/// A single token parsed from an AAR codeline message.
#[derive(Debug, Clone, PartialEq)]
pub struct AarToken {
    pub symbol: String,
    pub asserted: bool,
}

/// Codec for Association of American Railroads (AAR) symbolic codeline tokens.
pub struct AarCodec;

impl AarCodec {
    /// Tokenizes a comma-separated AAR string into individual asserted or parenthesized tokens.
    pub fn tokenize(input: &str) -> Vec<AarToken> {
        let mut tokens = Vec::new();
        for raw_part in input.split(',') {
            let part = raw_part.trim();
            if part.is_empty() {
                continue;
            }
            if part.starts_with('(') && part.ends_with(')') {
                let inner = &part[1..part.len() - 1].trim();
                tokens.push(AarToken {
                    symbol: inner.to_uppercase(),
                    asserted: false,
                });
            } else {
                tokens.push(AarToken {
                    symbol: part.to_uppercase(),
                    asserted: true,
                });
            }
        }
        tokens
    }

    /// Parses an inbound AAR indication string ending in 'K' into an IndicationVector.
    pub fn parse_indications(payload: &str) -> IndicationVector {
        let mut vector = IndicationVector {
            raw: payload.to_string(),
            ..Default::default()
        };

        let tokens = Self::tokenize(payload);

        // Track intermediate NWK and RWK states to determine correspondence vs moving
        let mut nwk_states: HashMap<String, bool> = HashMap::new();
        let mut rwk_states: HashMap<String, bool> = HashMap::new();

        // Track intermediate SGK and NGK states for signals
        let mut sgk_states: HashMap<String, bool> = HashMap::new();
        let mut ngk_states: HashMap<String, bool> = HashMap::new();

        for token in tokens {
            let sym = &token.symbol;
            if !sym.ends_with('K') {
                continue;
            }

            // 1. Maintainer call (MC1K, MCK, etc.)
            if sym.starts_with("MC") {
                if token.asserted {
                    vector.maintainer_call = true;
                }
                continue;
            }

            // 2. Approach time lock (e.g. 2TEK, 4TEK)
            if sym.ends_with("TEK") {
                let id = sym.trim_end_matches("TEK").to_string();
                vector.time_locks.insert(id, token.asserted);
                continue;
            }

            // 3. Switch Normal Correspondence (NWK)
            if sym.ends_with("NWK") {
                let id = sym.trim_end_matches("NWK").to_string();
                nwk_states.insert(id, token.asserted);
                continue;
            }

            // 4. Switch Reverse Correspondence (RWK)
            if sym.ends_with("RWK") {
                let id = sym.trim_end_matches("RWK").to_string();
                rwk_states.insert(id, token.asserted);
                continue;
            }

            // 5. Signal Permissive (SGK / NGK)
            if sym.ends_with("SGK") {
                let id = sym.trim_end_matches("SGK").to_string();
                sgk_states.insert(id, token.asserted);
                continue;
            }
            if sym.ends_with("NGK") {
                let id = sym.trim_end_matches("NGK").to_string();
                ngk_states.insert(id, token.asserted);
                continue;
            }

            // 6. Track Circuit Occupancy (TK or ending in K like 1T1K, 1TK)
            if sym.ends_with("TK") {
                let id = sym.trim_end_matches('K').to_string();
                vector.tracks.insert(id, token.asserted);
                continue;
            }

            // Generic track circuit tag: e.g. "1T1K" -> circuit "1T1"
            let id = sym[..sym.len() - 1].to_string();
            vector.tracks.insert(id, token.asserted);
        }

        // Resolve switch correspondence states
        let mut all_switches: Vec<String> = nwk_states.keys().chain(rwk_states.keys()).cloned().collect();
        all_switches.sort();
        all_switches.dedup();

        for sw in all_switches {
            let nw = nwk_states.get(&sw).copied().unwrap_or(false);
            let rw = rwk_states.get(&sw).copied().unwrap_or(false);
            if nw && !rw {
                vector.switches.insert(sw, "Normal".to_string());
            } else if rw && !nw {
                vector.switches.insert(sw, "Reverse".to_string());
            } else if !nw && !rw {
                vector.switches.insert(sw, "Moving".to_string());
            } else {
                vector.switches.insert(sw, "Conflict".to_string());
            }
        }

        // Resolve signal authorities
        let mut all_signals: Vec<String> = sgk_states.keys().chain(ngk_states.keys()).cloned().collect();
        all_signals.sort();
        all_signals.dedup();

        for sig in all_signals {
            let sg = sgk_states.get(&sig).copied().unwrap_or(false);
            let ng = ngk_states.get(&sig).copied().unwrap_or(false);
            if sg && !ng {
                vector.signals.insert(sig, "Right".to_string());
            } else if ng && !sg {
                vector.signals.insert(sig, "Left".to_string());
            } else {
                vector.signals.insert(sig, "Stop".to_string());
            }
        }

        vector
    }

    /// Formats a ControlSnapshot into an AAR comma-separated string ending in 'S'.
    /// When `ordered_switches` and `ordered_signals` are provided, tokens follow exact declaration order.
    pub fn format_controls(
        ordered_switches: &[&str],
        ordered_signals: &[&str],
        snapshot: &ControlSnapshot,
    ) -> String {
        let mut tokens = Vec::new();

        // 1. Switches: NWS, RWS pairs
        for sw in ordered_switches {
            let demand = snapshot.switches.get(*sw).map(|s| s.as_str()).unwrap_or("NoChange");
            match demand {
                "Normal" => {
                    tokens.push(format!("{}NWS", sw));
                    tokens.push(format!("({}RWS)", sw));
                }
                "Reverse" => {
                    tokens.push(format!("({}NWS)", sw));
                    tokens.push(format!("{}RWS", sw));
                }
                _ => {
                    tokens.push(format!("({}NWS)", sw));
                    tokens.push(format!("({}RWS)", sw));
                }
            }
        }

        // 2. Signals: SGS (Right), NGS (Left), HS (Stop)
        for sig in ordered_signals {
            let demand = snapshot.signals.get(*sig).map(|s| s.as_str()).unwrap_or("NoChange");
            match demand {
                "Left" => {
                    tokens.push(format!("({}SGS)", sig));
                    tokens.push(format!("{}NGS", sig));
                    tokens.push(format!("({}HS)", sig));
                }
                "Right" => {
                    tokens.push(format!("{}SGS", sig));
                    tokens.push(format!("({}NGS)", sig));
                    tokens.push(format!("({}HS)", sig));
                }
                "Stop" => {
                    tokens.push(format!("({}SGS)", sig));
                    tokens.push(format!("({}NGS)", sig));
                    tokens.push(format!("{}HS", sig));
                }
                _ => {
                    tokens.push(format!("({}SGS)", sig));
                    tokens.push(format!("({}NGS)", sig));
                    tokens.push(format!("({}HS)", sig));
                }
            }
        }

        // 3. Optional Maintainer call
        if let Some(mc) = snapshot.maintainer_call {
            if mc {
                tokens.push("MC1S".to_string());
            } else {
                tokens.push("(MC1S)".to_string());
            }
        }

        tokens.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_asserted_and_unasserted() {
        let input = "1NWS, (1RWS), 3NWS, (3RWS), (2SGS), 2NGS, (2HS), MC1S";
        let tokens = AarCodec::tokenize(input);
        assert_eq!(tokens.len(), 8);
        assert_eq!(tokens[0], AarToken { symbol: "1NWS".to_string(), asserted: true });
        assert_eq!(tokens[1], AarToken { symbol: "1RWS".to_string(), asserted: false });
        assert_eq!(tokens[5], AarToken { symbol: "2NGS".to_string(), asserted: true });
        assert_eq!(tokens[7], AarToken { symbol: "MC1S".to_string(), asserted: true });
    }

    #[test]
    fn test_parse_fieldunit_baseline_indications() {
        // From test_wire_codec.cpp in FieldUnit
        let payload = "1NWK, (1RWK), (3NWK), 3RWK, 1T1K, (3T1K), (2SGK), 2NGK, (2TEK), MC1K";
        let vector = AarCodec::parse_indications(payload);

        assert_eq!(vector.switches.get("1").unwrap(), "Normal");
        assert_eq!(vector.switches.get("3").unwrap(), "Reverse");
        assert_eq!(vector.tracks.get("1T1").unwrap(), &true);
        assert_eq!(vector.tracks.get("3T1").unwrap(), &false);
        assert_eq!(vector.signals.get("2").unwrap(), "Left");
        assert_eq!(vector.time_locks.get("2").unwrap(), &false);
        assert!(vector.maintainer_call);
    }

    #[test]
    fn test_parse_moving_switch_correspondence() {
        // Both NWK and RWK unasserted means points in transit (Moving)
        let payload = "(1NWK), (1RWK), (1T1K)";
        let vector = AarCodec::parse_indications(payload);
        assert_eq!(vector.switches.get("1").unwrap(), "Moving");
        assert_eq!(vector.tracks.get("1T1").unwrap(), &false);
    }

    #[test]
    fn test_parse_time_locking_active() {
        let payload = "1NWK, (1RWK), (2SGK), (2NGK), 2TEK";
        let vector = AarCodec::parse_indications(payload);
        assert_eq!(vector.signals.get("2").unwrap(), "Stop");
        assert_eq!(vector.time_locks.get("2").unwrap(), &true);
    }

    #[test]
    fn test_format_controls_exact_sequence() {
        let mut snapshot = ControlSnapshot::default();
        snapshot.switches.insert("1".to_string(), "Normal".to_string());
        snapshot.switches.insert("3".to_string(), "Reverse".to_string());
        snapshot.signals.insert("2".to_string(), "Left".to_string());
        snapshot.maintainer_call = Some(false);

        let formatted = AarCodec::format_controls(&["1", "3"], &["2"], &snapshot);
        let expected = "1NWS, (1RWS), (3NWS), 3RWS, (2SGS), 2NGS, (2HS), (MC1S)";
        assert_eq!(formatted, expected);
    }
}
