# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **US&S Style 504 cTc Machine & Model Board** (`CtcConsole.svelte`):
  - Industrial olive green chassis enclosure with unified 1008px width across model board and lever console.
  - Authentic US&S pre-punched metal bay architecture across all 7 column positions.
  - US&S shield number plates with pot-lid and shoulder geometry behind switch and signal levers.
  - Large, bold white `N`/`R` and `L`/`STOP`/`R` indications positioned under jewel lamps.
  - Proper US&S switch lever angles (-30° Normal / +30° Reverse) and signal lever angles (-30° L / 0° STOP / +30° R).
  - Searchlight signal repeater heads oriented horizontally along track paths.
  - Interlocked Derail 5 operation coupled with Switch 1.
- **MQTT Interface "A" Codeline Integration** (`src-tauri`, `CtcConsole.svelte`):
  - Added `rumqttc` async MQTT client to the Rust backend.
  - Implemented `AarCodec` (`src-tauri/src/core/aar_codec.rs`) for parsing AAR indication tokens (`*K`) and formatting AAR control tokens (`*S`).
  - Added 5 unit tests validating AAR token parsing, moving switch correspondence, time lock detection, and control sequence formatting.
  - Implemented `MqttCodelineManager` (`src-tauri/src/core/mqtt_codeline.rs`) subscribing to `/layout/<name>/codeline/<cp>/[indications, json, telemetry]` and publishing to `.../controls`.
  - Added Studio Role 4: authoritative publication of retained plant JSON specifications to `.../json`, plus discovery and read APIs (`codeline_get_plant_json`, `codeline_list_known_plants`) for loading existing broker configurations.
  - Wired `CtcConsole.svelte` to live `codeline:indication` events and code button control transmission, with graceful fallback to standalone local operation when no broker is present.
