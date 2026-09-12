<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { studio } from '$lib/state.svelte';

  // Codeline MQTT connection state
  let codelineStatus = $state<'Connected' | 'Connecting' | 'Disconnected' | string>('Connecting');
  let codelineLayout = $state('spcoast');
  let activeCp = $state('CP_Corporal');

  // Lever Demands (Dispatcher intent on the console deck)
  let switchDemands = $state<Record<string, 'Normal' | 'Reverse'>>({
    '1': 'Normal',
    '3': 'Normal',
    '5': 'Normal',
  });

  let signalDemands = $state<Record<string, 'Left' | 'Stop' | 'Right'>>({
    '2': 'Stop',
    '4': 'Stop',
  });

  // Blocking Dogs on levers (Red = Out of Service, Blue = Blue Flag Protection)
  let switchDogs = $state<Record<string, 'none' | 'red' | 'blue'>>({
    '1': 'none',
    '3': 'none',
    '5': 'none',
  });

  let signalDogs = $state<Record<string, 'none' | 'red' | 'blue'>>({
    '2': 'none',
    '4': 'none',
  });

  // Field Truth (Reported indications from the autonomous plant)
  let switchFieldStatus = $state<Record<string, 'Normal' | 'Reverse' | 'Moving'>>({
    '1': 'Normal',
    '3': 'Normal',
    '5': 'Normal',
  });

  let trackOccupancy = $state<Record<string, boolean>>({
    '2SAT': false,
    '1SAT': false,
    '1T1': false,
    '3T1': false,
    '5T1': false,
    '1NAT': false,
    'IND1': false,
  });

  let signalAspects = $state<Record<string, 'Stop' | 'Clear' | 'Diverging' | 'Restricting'>>({
    '2NAB': 'Stop',
    '2SA': 'Stop',
    '4NA': 'Stop',
    '4SA': 'Stop',
  });

  // Time Locking countdown timers (in seconds)
  let timeLockSeconds = $state<Record<string, number>>({
    '2': 0,
    '4': 0,
  });

  let timeLockInterval: number | null = null;
  let transitAlarmActive = $state(false);

  onMount(() => {
    let unlistenIndication: UnlistenFn | null = null;
    let unlistenStatus: UnlistenFn | null = null;

    // 1. Listen for inbound AAR indications from MQTT Interface "A"
    listen<{
      cp_name: string;
      vector: {
        switches: Record<string, string>;
        tracks: Record<string, boolean>;
        signals: Record<string, string>;
        time_locks: Record<string, boolean>;
        maintainer_call: boolean;
      };
    }>('codeline:indication', (event) => {
      const v = event.payload.vector;
      if (v.switches) {
        for (const [sw, pos] of Object.entries(v.switches)) {
          if (pos === 'Normal' || pos === 'Reverse' || pos === 'Moving') {
            switchFieldStatus[sw] = pos;
            if (sw === '1') switchFieldStatus['5'] = pos;
          }
        }
      }
      if (v.tracks) {
        for (const [tc, occ] of Object.entries(v.tracks)) {
          trackOccupancy[tc] = occ;
        }
      }
      if (v.signals) {
        for (const [sig, auth] of Object.entries(v.signals)) {
          if (sig === '2') {
            if (auth === 'Left') {
              signalAspects['2NAB'] = switchFieldStatus['3'] === 'Reverse' ? 'Diverging' : 'Clear';
              signalAspects['2SA'] = 'Stop';
            } else if (auth === 'Right') {
              signalAspects['2SA'] = 'Clear';
              signalAspects['2NAB'] = 'Stop';
            } else {
              signalAspects['2NAB'] = 'Stop';
              signalAspects['2SA'] = 'Stop';
            }
          } else if (sig === '4') {
            if (auth === 'Left') {
              signalAspects['4NA'] = 'Restricting';
              signalAspects['4SA'] = 'Stop';
            } else if (auth === 'Right') {
              signalAspects['4SA'] = 'Restricting';
              signalAspects['4NA'] = 'Stop';
            } else {
              signalAspects['4NA'] = 'Stop';
              signalAspects['4SA'] = 'Stop';
            }
          }
        }
      }
      if (v.time_locks) {
        for (const [sig, tl] of Object.entries(v.time_locks)) {
          timeLockSeconds[sig] = tl ? 15 : 0;
        }
      }
    }).then((fn) => {
      unlistenIndication = fn;
    });

    // 2. Listen for codeline connection state changes
    listen<string>('codeline:status', (event) => {
      codelineStatus = event.payload;
    }).then((fn) => {
      unlistenStatus = fn;
    });

    // 3. Connect to MQTT broker (localhost:1883 or LAN broker)
    invoke('codeline_connect', {
      host: 'localhost',
      port: 1883,
      layout: codelineLayout,
    }).catch((err) => {
      console.info('MQTT broker not reachable on localhost:1883, using standalone local mode:', err);
      codelineStatus = 'Disconnected';
    });

    // 4. Local time lock countdown ticker
    timeLockInterval = window.setInterval(() => {
      let active = false;
      for (const sig of ['2', '4']) {
        if (timeLockSeconds[sig] > 0) {
          timeLockSeconds[sig] -= 1;
          active = true;
        }
      }
      transitAlarmActive = active;
    }, 1000);

    return () => {
      if (timeLockInterval !== null) clearInterval(timeLockInterval);
      if (unlistenIndication) unlistenIndication();
      if (unlistenStatus) unlistenStatus();
    };
  });

  // Toggle switch lever (US&S 2-position: Normal UP 0° ↔ Reverse DOWN 180°)
  function toggleSwitchLever(swId: string) {
    if (switchDogs[swId] !== 'none') return;
    const next = switchDemands[swId] === 'Normal' ? 'Reverse' : 'Normal';
    switchDemands = { ...switchDemands, [swId]: next };
  }

  // Cycle signal lever (US&S 3-position: Left -45° ↔ Stop 0° ↔ Right +45°)
  function cycleSignalLever(sigId: string) {
    if (signalDogs[sigId] !== 'none') return;
    const current = signalDemands[sigId];
    const next = current === 'Stop' ? 'Left' : current === 'Left' ? 'Right' : 'Stop';
    signalDemands = { ...signalDemands, [sigId]: next };
  }

  // Clamp / remove mechanical blocking dog on right click
  function cycleDog(type: 'switch' | 'signal', id: string, event: MouseEvent) {
    event.preventDefault();
    if (type === 'switch') {
      const curr = switchDogs[id];
      const next = curr === 'none' ? 'red' : curr === 'red' ? 'blue' : 'none';
      switchDogs = { ...switchDogs, [id]: next };
    } else {
      const curr = signalDogs[id];
      const next = curr === 'none' ? 'red' : curr === 'red' ? 'blue' : 'none';
      signalDogs = { ...signalDogs, [id]: next };
    }
  }

  // Toggle track occupancy (interactive shunt testing)
  function toggleShunt(circuitId: string) {
    trackOccupancy = { ...trackOccupancy, [circuitId]: !trackOccupancy[circuitId] };

    // Vital safety: if train shunts an island while signal is clear, instant knockdown to Stop!
    if (trackOccupancy['3T1'] || trackOccupancy['1T1']) {
      signalAspects = { ...signalAspects, '2NAB': 'Stop', '2SA': 'Stop' };
    }
    if (trackOccupancy['5T1'] || trackOccupancy['1T1']) {
      signalAspects = { ...signalAspects, '4NA': 'Stop', '4SA': 'Stop' };
    }
  }

  // Transmit atomic control snapshot for a station column (Code Button press)
  async function punchCodeButton(stationCol: number) {
    const swId = stationCol === 1 ? '1' : '3';
    const sigId = stationCol === 1 ? '4' : '2';

    const switchesToTransmit = stationCol === 1 ? ['1', '5'] : ['3'];
    const signalsToTransmit = stationCol === 1 ? ['4'] : ['2'];

    // If connected to MQTT broker, publish live Interface "A" control tokens
    if (codelineStatus === 'Connected') {
      try {
        await invoke('codeline_publish_controls', {
          cpName: activeCp,
          switches: switchesToTransmit,
          signals: signalsToTransmit,
          snapshot: {
            cp_name: activeCp,
            switches: {
              [swId]: switchDemands[swId],
              ...(swId === '1' ? { '5': switchDemands['1'] } : {}),
            },
            signals: {
              [sigId]: signalDemands[sigId],
            },
            maintainer_call: false,
          },
        });
        return;
      } catch (err) {
        console.warn('MQTT publish failed, falling back to local simulation:', err);
      }
    }

    // Fallback: local standalone simulation when no broker or field unit is active
    const demandedPos = switchDemands[swId];
    const islandLocked = swId === '1' 
      ? (trackOccupancy['1T1'] || trackOccupancy['5T1']) 
      : trackOccupancy['3T1'];
    const isLocked = islandLocked || (timeLockSeconds[sigId] > 0);

    if (!isLocked && switchFieldStatus[swId] !== demandedPos) {
      if (swId === '1') {
        switchFieldStatus = { ...switchFieldStatus, '1': 'Moving', '5': 'Moving' };
        setTimeout(() => {
          switchFieldStatus = { ...switchFieldStatus, '1': demandedPos, '5': demandedPos };
          evaluatePlantRoutes();
        }, 2000);
      } else {
        switchFieldStatus = { ...switchFieldStatus, '3': 'Moving' };
        setTimeout(() => {
          switchFieldStatus = { ...switchFieldStatus, '3': demandedPos };
          evaluatePlantRoutes();
        }, 2000);
      }
    }

    const demandedSig = signalDemands[sigId];
    if (sigId === '2' && demandedSig === 'Stop' && (signalAspects['2NAB'] !== 'Stop' || signalAspects['2SA'] !== 'Stop')) {
      timeLockSeconds = { ...timeLockSeconds, [sigId]: 15 };
    } else if (sigId === '4' && demandedSig === 'Stop' && (signalAspects['4NA'] !== 'Stop' || signalAspects['4SA'] !== 'Stop')) {
      timeLockSeconds = { ...timeLockSeconds, [sigId]: 15 };
    }

    evaluatePlantRoutes();
  }

  // Evaluate Interlocking Control Table rules
  function evaluatePlantRoutes() {
    // Route 1: MT-NB (Main 2 Northbound, Signal 2 Left)
    if (
      signalDemands['2'] === 'Left' &&
      switchFieldStatus['1'] === 'Normal' &&
      switchFieldStatus['3'] === 'Normal' &&
      !trackOccupancy['3T1'] &&
      !trackOccupancy['1T1'] &&
      !trackOccupancy['2SAT'] &&
      timeLockSeconds['2'] === 0
    ) {
      signalAspects['2NAB'] = 'Clear';
    } else if (
      // Route 2: MT-NB-REV (Main 2 to Main 1, Switch 3 Reverse)
      signalDemands['2'] === 'Left' &&
      switchFieldStatus['3'] === 'Reverse' &&
      !trackOccupancy['3T1'] &&
      !trackOccupancy['1SAT'] &&
      timeLockSeconds['2'] === 0
    ) {
      signalAspects['2NAB'] = 'Diverging';
    } else {
      signalAspects['2NAB'] = 'Stop';
    }

    // Route 3: SB-MT (MT1 Southbound, Signal 2 Right)
    if (
      signalDemands['2'] === 'Right' &&
      switchFieldStatus['3'] === 'Reverse' &&
      !trackOccupancy['3T1'] &&
      !trackOccupancy['1NAT'] &&
      timeLockSeconds['2'] === 0
    ) {
      signalAspects['2SA'] = 'Clear';
    } else {
      signalAspects['2SA'] = 'Stop';
    }

    // Route 4: IND-NB (Industry Lead Northbound, Signal 4 Left)
    if (
      signalDemands['4'] === 'Left' &&
      switchFieldStatus['1'] === 'Reverse' &&
      switchFieldStatus['5'] === 'Reverse' &&
      !trackOccupancy['1T1'] &&
      !trackOccupancy['5T1']
    ) {
      signalAspects['4NA'] = 'Restricting';
    } else {
      signalAspects['4NA'] = 'Stop';
    }
  }
</script>

<div class="ctc-desk-root">
    <!-- Faceplate Header Banner -->
    <div class="faceplate-banner">
      <div class="banner-title">SOUTHERN PACIFIC COAST DIVISION — CP CORPORAL (MP 83.2)</div>
      <div class="banner-status">
        CODELINE:
        {#if codelineStatus === 'Connected'}
          <span class="text-green font-bold">CONNECTED ({codelineLayout} / {activeCp})</span>
        {:else if codelineStatus === 'Connecting'}
          <span class="text-amber font-bold">CONNECTING...</span>
        {:else}
          <span class="text-slate font-bold">STANDALONE (LOCAL)</span>
        {/if}
        |
        {#if transitAlarmActive}
          <span class="text-red font-bold animate-pulse">ALARM: TIME LOCK RUNNING (2TEK)</span>
        {:else}
          <span class="text-green">STATUS: NORMAL CORRESPONDENCE</span>
        {/if}
      </div>
    </div>

    <!-- UPPER SECTION: The US&S Model Board (John Signor SP Style) -->
    <div class="model-board-section">
      <div class="model-board-frame">
        <svg class="model-board-svg" viewBox="0 0 1008 260">
          <defs>
            <!-- Faceted Glass Jewel Lamp Gradient: Red (Occupancy/Stop) -->
            <radialGradient id="jewel-red-lit" cx="35%" cy="35%" r="65%">
              <stop offset="0%" stop-color="#fca5a5" />
              <stop offset="40%" stop-color="#ef4444" />
              <stop offset="85%" stop-color="#b91c1c" />
              <stop offset="100%" stop-color="#450a0a" />
            </radialGradient>
            <radialGradient id="jewel-red-dark" cx="35%" cy="35%" r="65%">
              <stop offset="0%" stop-color="#7f1d1d" />
              <stop offset="70%" stop-color="#450a0a" />
              <stop offset="100%" stop-color="#1c0505" />
            </radialGradient>

            <!-- Faceted Glass Jewel: Opal / White (Route / Normal) -->
            <radialGradient id="jewel-white-lit" cx="35%" cy="35%" r="65%">
              <stop offset="0%" stop-color="#ffffff" />
              <stop offset="40%" stop-color="#f1f5f9" />
              <stop offset="85%" stop-color="#cbd5e1" />
              <stop offset="100%" stop-color="#475569" />
            </radialGradient>
            <radialGradient id="jewel-white-dark" cx="35%" cy="35%" r="65%">
              <stop offset="0%" stop-color="#475569" />
              <stop offset="70%" stop-color="#1e293b" />
              <stop offset="100%" stop-color="#0f172a" />
            </radialGradient>

            <!-- Faceted Glass Jewel: Amber (Reverse) -->
            <radialGradient id="jewel-amber-lit" cx="35%" cy="35%" r="65%">
              <stop offset="0%" stop-color="#fef08a" />
              <stop offset="40%" stop-color="#f59e0b" />
              <stop offset="85%" stop-color="#d97706" />
              <stop offset="100%" stop-color="#78350f" />
            </radialGradient>
            <radialGradient id="jewel-amber-dark" cx="35%" cy="35%" r="65%">
              <stop offset="0%" stop-color="#78350f" />
              <stop offset="70%" stop-color="#291004" />
              <stop offset="100%" stop-color="#0c0401" />
            </radialGradient>

            <!-- Faceted Glass Jewel: Green (Permissive) -->
            <radialGradient id="jewel-green-lit" cx="35%" cy="35%" r="65%">
              <stop offset="0%" stop-color="#86efac" />
              <stop offset="40%" stop-color="#22c55e" />
              <stop offset="85%" stop-color="#15803d" />
              <stop offset="100%" stop-color="#052e16" />
            </radialGradient>
            <radialGradient id="jewel-green-dark" cx="35%" cy="35%" r="65%">
              <stop offset="0%" stop-color="#14532d" />
              <stop offset="70%" stop-color="#052e16" />
              <stop offset="100%" stop-color="#02150a" />
            </radialGradient>

            <!-- Chrome Bezel Ring Filter -->
            <filter id="chrome-glow" x="-20%" y="-20%" width="140%" height="140%">
              <feDropShadow dx="0" dy="1" stdDeviation="1.5" flood-color="#000000" flood-opacity="0.8" />
            </filter>

            <!-- Large Molded Bakelite Paddle Handle Gradient -->
            <linearGradient id="paddle-plastic" x1="0%" y1="0%" x2="100%" y2="0%">
              <stop offset="0%" stop-color="#0f172a" />
              <stop offset="25%" stop-color="#334155" />
              <stop offset="50%" stop-color="#64748b" />
              <stop offset="75%" stop-color="#334155" />
              <stop offset="100%" stop-color="#0f172a" />
            </linearGradient>

            <!-- Machined Chrome Center Pivot Hub Gradient -->
            <radialGradient id="hub-chrome" cx="35%" cy="35%" r="65%">
              <stop offset="0%" stop-color="#ffffff" />
              <stop offset="35%" stop-color="#e2e8f0" />
              <stop offset="75%" stop-color="#64748b" />
              <stop offset="100%" stop-color="#0f172a" />
            </radialGradient>

            <!-- Machined Round Code Button Gradient -->
            <radialGradient id="button-chrome" cx="35%" cy="35%" r="65%">
              <stop offset="0%" stop-color="#ffffff" />
              <stop offset="30%" stop-color="#cbd5e1" />
              <stop offset="70%" stop-color="#64748b" />
              <stop offset="100%" stop-color="#1e293b" />
            </radialGradient>

            <!-- Embossed Metal Shield Plate Gradient -->
            <linearGradient id="shield-plate-metal" x1="0%" y1="0%" x2="0%" y2="100%">
              <stop offset="0%" stop-color="#2a3440" />
              <stop offset="25%" stop-color="#18202a" />
              <stop offset="100%" stop-color="#0e1318" />
            </linearGradient>
          </defs>

          <!-- Board Surface Background (Black) -->
          <rect width="100%" height="100%" fill="#0a0d14" rx="4" />

          <!-- TOP BAND: Thin-Lined Plant Track Diagram & Geographic Features -->
          <g class="thin-schematic-band" opacity="0.6">
            <!-- MT2 (Runs all the way across) -->
            <line x1="50" y1="28" x2="958" y2="28" stroke="#cbd5e1" stroke-width="2" />
            <!-- MT1 merging into MT2 at SW3, turning 2MT to single mainline going south -->
            <path d="M 50,40 L 444,40 L 504,28" fill="none" stroke="#cbd5e1" stroke-width="2" stroke-linejoin="round" />
            <!-- Industry spur thin line -->
            <path d="M 300,28 L 360,16 L 680,16" fill="none" stroke="#94a3b8" stroke-width="1.5" />
            <!-- Highway Overpass Landmark -->
            <line x1="280" y1="8" x2="280" y2="48" stroke="#f59e0b" stroke-width="2" stroke-dasharray="3 3" />
            <line x1="310" y1="8" x2="310" y2="48" stroke="#f59e0b" stroke-width="2" stroke-dasharray="3 3" />
            <text x="295" y="10" text-anchor="middle" fill="#f59e0b" font-size="8" font-weight="700">US 101 OVERPASS</text>
            <!-- River/Creek Landmark -->
            <text x="730" y="10" text-anchor="middle" fill="#38bdf8" font-size="8" font-weight="700">CARNADERO CREEK</text>
            <path d="M 710,12 C 720,28 725,32 735,46" fill="none" stroke="#38bdf8" stroke-width="1.5" stroke-dasharray="2 2" />
          </g>

          <!-- ROW 2: Control Point Names Row (No Boxes Around Them!) -->
          <text x="504" y="66" text-anchor="middle" fill="#ffffff" font-size="16" font-weight="900" letter-spacing="2.5px" font-family="'Times New Roman', serif">
            CP CORPORAL
          </text>

          <!-- ROW 3: Thick-Lined Model Board (12px Solid White Lines, Embedded Jewels, Clean Path Joins) -->

          <!-- LEVEL -1: Industry Lead (5T1 & IND1) -->
          <!-- Crossover branch from Switch 1 cleanly joined using M/L path -->
          <path d="M 300,145 L 390,105 L 620,105" fill="none" stroke="#ffffff" stroke-width="12" stroke-linejoin="round" stroke-linecap="butt" />
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <g class="track-block" onclick={() => toggleShunt('5T1')}>
            <circle cx="505" cy="105" r="8" fill={trackOccupancy['5T1'] ? 'url(#jewel-red-lit)' : 'url(#jewel-red-dark)'} stroke="#94a3b8" stroke-width="2" filter="url(#chrome-glow)" />
            <text x="505" y="90" text-anchor="middle" fill={trackOccupancy['5T1'] ? '#ef4444' : '#94a3b8'} font-size="10" font-family="monospace" font-weight="800">5T1</text>
          </g>

          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <g class="track-block" onclick={() => toggleShunt('IND1')}>
            <line x1="620" y1="105" x2="800" y2="105" stroke="#ffffff" stroke-width="12" />
            <!-- Red Bumping Post -->
            <rect x="800" y="93" width="8" height="24" fill="#ef4444" stroke="#ffffff" stroke-width="2" rx="1" />
            <circle cx="710" cy="105" r="8" fill={trackOccupancy['IND1'] ? 'url(#jewel-red-lit)' : 'url(#jewel-red-dark)'} stroke="#94a3b8" stroke-width="2" filter="url(#chrome-glow)" />
            <text x="710" y="90" text-anchor="middle" fill="#94a3b8" font-size="10" font-family="monospace" font-weight="700">IND 1</text>
          </g>

          <!-- Derail 5 Stamped ID & Point Indicator Lamp (Interlocked with SW1) -->
          <text x="365" y="94" fill="#f8fafc" font-size="13" font-weight="900" font-family="sans-serif">5</text>
          <circle cx="390" cy="105" r="7" fill={switchFieldStatus['1'] === 'Normal' ? '#ef4444' : '#ffffff'} stroke="#000000" stroke-width="2" />

          <!-- Signal 4NA Searchlight Head (Rotated CCW 90°: Horizontal along track) -->
          <line x1="620" y1="105" x2="620" y2="92" stroke="#cbd5e1" stroke-width="2" />
          <line x1="620" y1="92" x2="638" y2="92" stroke="#cbd5e1" stroke-width="2.5" />
          <circle cx="638" cy="92" r="7" fill={signalAspects['4NA'] !== 'Stop' ? 'url(#jewel-green-lit)' : 'url(#jewel-red-lit)'} stroke="#ffffff" stroke-width="1.5" />
          <text x="652" y="96" fill="#e2e8f0" font-size="10" font-weight="800">4NA</text>

          <!-- LEVEL 0: Mainline MT2 (Northbound / Eastward) -->
          <text x="50" y="130" fill="#94a3b8" font-size="11" font-weight="900" letter-spacing="0.5">MAIN 2</text>

          <!-- Approach Block 2SAT -->
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <g class="track-block" onclick={() => toggleShunt('2SAT')}>
            <line x1="100" y1="145" x2="200" y2="145" stroke="#ffffff" stroke-width="12" stroke-linecap="round" />
            <circle cx="150" cy="145" r="8" fill={trackOccupancy['2SAT'] ? 'url(#jewel-red-lit)' : 'url(#jewel-red-dark)'} stroke="#94a3b8" stroke-width="2" filter="url(#chrome-glow)" />
            <text x="150" y="130" text-anchor="middle" fill={trackOccupancy['2SAT'] ? '#ef4444' : '#94a3b8'} font-size="10" font-family="monospace" font-weight="800">2SAT</text>
          </g>

          <!-- Signal 4SA on MT2 (Rotated CCW 90°: Horizontal along track facing left) -->
          <line x1="200" y1="145" x2="200" y2="158" stroke="#cbd5e1" stroke-width="2" />
          <line x1="200" y1="158" x2="182" y2="158" stroke="#cbd5e1" stroke-width="2.5" />
          <circle cx="182" cy="158" r="7" fill={signalAspects['4SA'] !== 'Stop' ? 'url(#jewel-green-lit)' : 'url(#jewel-red-lit)'} stroke="#ffffff" stroke-width="1.5" />
          <text x="182" y="174" text-anchor="middle" fill="#e2e8f0" font-size="10" font-weight="800">4SA</text>

          <!-- Main 2 Continuous Line through Interlocking -->
          <line x1="200" y1="145" x2="720" y2="145" stroke="#ffffff" stroke-width="12" />

          <!-- Island Block 1T1 -->
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <g class="track-block" onclick={() => toggleShunt('1T1')}>
            <circle cx="250" cy="145" r="8" fill={trackOccupancy['1T1'] ? 'url(#jewel-red-lit)' : 'url(#jewel-red-dark)'} stroke="#94a3b8" stroke-width="2" filter="url(#chrome-glow)" />
            <text x="250" y="130" text-anchor="middle" fill={trackOccupancy['1T1'] ? '#ef4444' : '#94a3b8'} font-size="10" font-family="monospace" font-weight="800">1T1</text>
          </g>

          <!-- Switch 1 Stamped ID & Route Indicator Point Lamps on Track -->
          <text x="286" y="130" fill="#f8fafc" font-size="13" font-weight="900" font-family="sans-serif">1</text>
          <circle cx="320" cy="145" r="5.5" fill={switchFieldStatus['1'] === 'Normal' ? '#ffffff' : '#1e293b'} stroke="#64748b" stroke-width="1.5" />
          <circle cx="320" cy="133" r="5.5" fill={switchFieldStatus['1'] === 'Reverse' ? '#f59e0b' : '#1e293b'} stroke="#64748b" stroke-width="1.5" />

          <!-- Switch 3 Diagonal Crossover Down to MT1 (Seamless path, butt ends inside Main 2 and Main 1) -->
          <path d="M 504,145 L 420,205" fill="none" stroke="#ffffff" stroke-width="12" stroke-linecap="butt" />

          <!-- Island Block 3T1 -->
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <g class="track-block" onclick={() => toggleShunt('3T1')}>
            <circle cx="610" cy="145" r="8" fill={trackOccupancy['3T1'] ? 'url(#jewel-red-lit)' : 'url(#jewel-red-dark)'} stroke="#94a3b8" stroke-width="2" filter="url(#chrome-glow)" />
            <text x="610" y="130" text-anchor="middle" fill={trackOccupancy['3T1'] ? '#ef4444' : '#94a3b8'} font-size="10" font-family="monospace" font-weight="800">3T1</text>
          </g>

          <!-- Switch 3 Stamped ID & Route Indicator Point Lamps on Track -->
          <text x="516" y="130" fill="#f8fafc" font-size="13" font-weight="900" font-family="sans-serif">3</text>
          <circle cx="490" cy="145" r="5.5" fill={switchFieldStatus['3'] === 'Normal' ? '#ffffff' : '#1e293b'} stroke="#64748b" stroke-width="1.5" />
          <circle cx="490" cy="157" r="5.5" fill={switchFieldStatus['3'] === 'Reverse' ? '#f59e0b' : '#1e293b'} stroke="#64748b" stroke-width="1.5" />

          <!-- Signal 2NAB on MT2 (Two Searchlight Heads, Rotated CCW 90°: Horizontal along track facing right) -->
          <line x1="720" y1="145" x2="720" y2="134" stroke="#cbd5e1" stroke-width="2" />
          <line x1="720" y1="134" x2="752" y2="134" stroke="#cbd5e1" stroke-width="2.5" />
          <circle cx="738" cy="134" r="6" fill={signalAspects['2NAB'] === 'Clear' ? 'url(#jewel-green-lit)' : 'url(#jewel-red-lit)'} stroke="#ffffff" stroke-width="1.5" />
          <circle cx="752" cy="134" r="6" fill={signalAspects['2NAB'] === 'Diverging' ? 'url(#jewel-green-lit)' : 'url(#jewel-red-lit)'} stroke="#ffffff" stroke-width="1.5" />
          <text x="764" y="138" fill="#e2e8f0" font-size="10" font-weight="800">2NAB</text>

          <!-- Single Track Main Exit 1NAT -->
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <g class="track-block" onclick={() => toggleShunt('1NAT')}>
            <line x1="720" y1="145" x2="950" y2="145" stroke="#ffffff" stroke-width="12" stroke-linecap="round" />
            <circle cx="830" cy="145" r="8" fill={trackOccupancy['1NAT'] ? 'url(#jewel-red-lit)' : 'url(#jewel-red-dark)'} stroke="#94a3b8" stroke-width="2" filter="url(#chrome-glow)" />
            <text x="830" y="130" text-anchor="middle" fill={trackOccupancy['1NAT'] ? '#ef4444' : '#94a3b8'} font-size="10" font-family="monospace" font-weight="800">1NAT</text>
          </g>

          <!-- LEVEL 1: Mainline MT1 (Southbound / Westward) -->
          <text x="50" y="195" fill="#94a3b8" font-size="11" font-weight="900" letter-spacing="0.5">MAIN 1</text>

          <!-- Approach Block 1SAT -->
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <g class="track-block" onclick={() => toggleShunt('1SAT')}>
            <line x1="100" y1="205" x2="200" y2="205" stroke="#ffffff" stroke-width="12" stroke-linecap="round" />
            <line x1="200" y1="205" x2="420" y2="205" stroke="#ffffff" stroke-width="12" />
            <circle cx="150" cy="205" r="8" fill={trackOccupancy['1SAT'] ? 'url(#jewel-red-lit)' : 'url(#jewel-red-dark)'} stroke="#94a3b8" stroke-width="2" filter="url(#chrome-glow)" />
            <text x="150" y="190" text-anchor="middle" fill={trackOccupancy['1SAT'] ? '#ef4444' : '#94a3b8'} font-size="10" font-family="monospace" font-weight="800">1SAT</text>
          </g>

          <!-- Signal 2SA Dwarf Searchlight on MT1 (Rotated CCW 90°: Horizontal along track facing left) -->
          <line x1="200" y1="205" x2="200" y2="218" stroke="#cbd5e1" stroke-width="2" />
          <line x1="200" y1="218" x2="182" y2="218" stroke="#cbd5e1" stroke-width="2.5" />
          <circle cx="182" cy="218" r="6" fill={signalAspects['2SA'] !== 'Stop' ? 'url(#jewel-green-lit)' : 'url(#jewel-red-lit)'} stroke="#ffffff" stroke-width="1.5" />
          <text x="182" y="234" text-anchor="middle" fill="#e2e8f0" font-size="10" font-weight="800">2SA</text>

        </svg>
      </div>
    </div>

    <!-- LOWER SECTION: Seamless Continuous US&S Style 504 Lever Deck (Olive Green, All Columns Punched, Same Width as Model Board) -->
    <div class="lever-deck-section">
      <div class="uss-continuous-console">
        <!-- Loop over 7 columns (0 through 6) - All columns share the exact same punched holes! -->
        {#each [0, 1, 2, 3, 4, 5, 6] as colIndex}
          <div class="uss-column-bay" class:unused-column={colIndex !== 1 && colIndex !== 2}>
            <svg viewBox="0 0 144 490" class="uss-column-svg">
              <!-- Background Pre-Punched Empty Holes (Identical on ALL columns!) -->
              <g class="punched-holes-layer">
                <circle cx="72" cy="14" r="4.5" fill="#080c12" stroke="#1e2918" stroke-width="1.5" />
                <!-- Switch Lamp Holes at (36, 40) and (108, 40) -->
                <circle cx="36" cy="40" r="9" fill="#080c12" stroke="#1e2918" stroke-width="2" />
                <circle cx="108" cy="40" r="9" fill="#080c12" stroke="#1e2918" stroke-width="2" />
                <!-- Switch Lever Shaft Hole at (72, 144) -->
                <circle cx="72" cy="144" r="14" fill="#080c12" stroke="#1e2918" stroke-width="2" />
                <circle cx="72" cy="144" r="4" fill="#040609" />
                <!-- Mid Mounting Screw at (72, 180) -->
                <circle cx="72" cy="180" r="4.5" fill="#080c12" stroke="#1e2918" stroke-width="1.5" />
                <!-- Signal STOP Lamp Hole at (72, 206) -->
                <circle cx="72" cy="206" r="9" fill="#080c12" stroke="#1e2918" stroke-width="2" />
                <!-- Signal L and R Lamp Holes at (36, 234) and (108, 234) -->
                <circle cx="36" cy="234" r="9" fill="#080c12" stroke="#1e2918" stroke-width="2" />
                <circle cx="108" cy="234" r="9" fill="#080c12" stroke="#1e2918" stroke-width="2" />
                <!-- Signal Lever Shaft Hole at (72, 336) -->
                <circle cx="72" cy="336" r="14" fill="#080c12" stroke="#1e2918" stroke-width="2" />
                <circle cx="72" cy="336" r="4" fill="#040609" />
                <!-- Code Button Hole at (72, 436) -->
                <circle cx="72" cy="436" r="18" fill="#080c12" stroke="#1e2918" stroke-width="2" />
              </g>

              <!-- ACTIVE COLUMN 1: Switch 1 & Signal 4 & Code 1 -->
              {#if colIndex === 1}
                <!-- Switch 1 Lamps (Directly seated in the punched holes at y=40) -->
                <circle cx="36" cy="40" r="8" fill={switchFieldStatus['1'] === 'Normal' ? 'url(#jewel-green-lit)' : 'url(#jewel-green-dark)'} stroke="#cbd5e1" stroke-width="1.5" filter="url(#chrome-glow)" />
                <circle cx="108" cy="40" r="8" fill={switchFieldStatus['1'] === 'Reverse' ? 'url(#jewel-amber-lit)' : 'url(#jewel-amber-dark)'} stroke="#cbd5e1" stroke-width="1.5" filter="url(#chrome-glow)" />

                <!-- Switch 1 US&S Shield Plate & Lever (Clickable) -->
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <g class="uss-plate-group" role="button" tabindex="0"
                  onclick={() => toggleSwitchLever('1')}
                  oncontextmenu={(e) => cycleDog('switch', '1', e)}
                >
                  <!-- Shield Plate (Solid Black with Crisp White/Silver Border) -->
                  <path
                    d="M 50,62 L 94,62 Q 102,62 106,70 L 122,86 Q 128,92 120,102 L 88,144 Q 78,156 72,156 Q 66,156 56,144 L 24,102 Q 16,92 22,86 L 38,70 Q 42,62 50,62 Z"
                    fill="#0a0a0a"
                    stroke="#ffffff"
                    stroke-width="1.8"
                  />
                  <!-- Stamped Number & Type in Raised Pot Lid -->
                  <text x="72" y="75" text-anchor="middle" fill="#ffffff" font-size="14" font-weight="900" font-family="'Arial', sans-serif">1</text>
                  <text x="72" y="85" text-anchor="middle" fill="#cbd5e1" font-size="7.5" font-weight="800" letter-spacing="1">SWITCH</text>

                  <!-- Unanimated Bold White Letters on Shoulders -->
                  <text x="36" y="99" text-anchor="middle" fill="#ffffff" font-size="14" font-weight="900" font-family="'Arial', sans-serif">N</text>
                  <text x="108" y="99" text-anchor="middle" fill="#ffffff" font-size="14" font-weight="900" font-family="'Arial', sans-serif">R</text>

                  <!-- Lever Paddle (Pivots at x=72, y=144; tip never covers text!) -->
                  <g class="uss-lever-rotor" transform="translate(72, 144) rotate({switchDemands['1'] === 'Normal' ? -30 : 30})">
                    <path
                      d="M -7,0 C -10,-14 -8,-36 -3,-46 C -1,-50 1,-50 3,-46 C 8,-36 10,-14 7,0 C 4,6 -4,6 -7,0 Z"
                      fill="url(#paddle-plastic)"
                      stroke="#0f172a"
                      stroke-width="1.5"
                    />
                    <line x1="0" y1="-46" x2="0" y2="-6" stroke="#f8fafc" stroke-width="2.5" stroke-linecap="round" />
                    <circle cx="0" cy="0" r="14" fill="url(#hub-chrome)" stroke="#090d16" stroke-width="1.5" />
                    <circle cx="0" cy="0" r="5" fill="#1e293b" />
                  </g>
                </g>

                <!-- Signal 4 Lamps in 2 Layers (Directly seated in punched holes) -->
                <!-- Layer 1: STOP Lamp at (72, 206) -->
                <circle cx="72" cy="206" r="8" fill={signalAspects['4NA'] === 'Stop' && signalAspects['4SA'] === 'Stop' ? 'url(#jewel-red-lit)' : 'url(#jewel-red-dark)'} stroke="#cbd5e1" stroke-width="1.5" filter="url(#chrome-glow)" />
                <!-- Layer 2: L and R Lamps at (36, 234) and (108, 234) -->
                <circle cx="36" cy="234" r="8" fill={signalAspects['4NA'] !== 'Stop' ? 'url(#jewel-green-lit)' : 'url(#jewel-green-dark)'} stroke="#cbd5e1" stroke-width="1.5" filter="url(#chrome-glow)" />
                <circle cx="108" cy="234" r="8" fill={signalAspects['4SA'] !== 'Stop' ? 'url(#jewel-green-lit)' : 'url(#jewel-green-dark)'} stroke="#cbd5e1" stroke-width="1.5" filter="url(#chrome-glow)" />

                <!-- Signal 4 US&S Shield Plate & Lever (Clickable) -->
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <g class="uss-plate-group" role="button" tabindex="0"
                  onclick={() => cycleSignalLever('4')}
                  oncontextmenu={(e) => cycleDog('signal', '4', e)}
                >
                  <path
                    d="M 50,254 L 94,254 Q 102,254 106,262 L 122,278 Q 128,284 120,294 L 88,336 Q 78,348 72,348 Q 66,348 56,336 L 24,294 Q 16,284 22,278 L 38,262 Q 42,254 50,254 Z"
                    fill="#0a0a0a"
                    stroke="#ffffff"
                    stroke-width="1.8"
                  />
                  <text x="72" y="267" text-anchor="middle" fill="#ffffff" font-size="14" font-weight="900" font-family="'Arial', sans-serif">4</text>
                  <text x="72" y="277" text-anchor="middle" fill="#cbd5e1" font-size="7.5" font-weight="800" letter-spacing="1">SIGNAL</text>

                  <text x="36" y="291" text-anchor="middle" fill="#ffffff" font-size="13" font-weight="900" font-family="'Arial', sans-serif">L</text>
                  <text x="72" y="290" text-anchor="middle" fill="#ffffff" font-size="8.5" font-weight="900" font-family="'Arial', sans-serif">STOP</text>
                  <text x="108" y="291" text-anchor="middle" fill="#ffffff" font-size="13" font-weight="900" font-family="'Arial', sans-serif">R</text>

                  <g class="uss-lever-rotor" transform="translate(72, 336) rotate({signalDemands['4'] === 'Left' ? -30 : signalDemands['4'] === 'Right' ? 30 : 0})">
                    <path
                      d="M -7,0 C -10,-14 -8,-36 -3,-46 C -1,-50 1,-50 3,-46 C 8,-36 10,-14 7,0 C 4,6 -4,6 -7,0 Z"
                      fill="url(#paddle-plastic)"
                      stroke="#0f172a"
                      stroke-width="1.5"
                    />
                    <line x1="0" y1="-46" x2="0" y2="-6" stroke="#f8fafc" stroke-width="2.5" stroke-linecap="round" />
                    <circle cx="0" cy="0" r="14" fill="url(#hub-chrome)" stroke="#090d16" stroke-width="1.5" />
                    <circle cx="0" cy="0" r="5" fill="#1e293b" />
                  </g>
                </g>

                <!-- Code Button 1 (Seated at 72, 436 - never cut off!) -->
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <g class="uss-code-button-interactive" role="button" tabindex="0" onclick={() => punchCodeButton(1)} cursor="pointer">
                  <circle cx="72" cy="436" r="21" fill="url(#button-chrome)" stroke="#94a3b8" stroke-width="2" />
                  <circle cx="72" cy="436" r="15" fill="url(#hub-chrome)" stroke="#475569" stroke-width="1" />
                  <text x="72" y="441" text-anchor="middle" fill="#0f172a" font-size="13" font-weight="900" font-family="'Arial', sans-serif">1</text>
                </g>
              {/if}

              <!-- ACTIVE COLUMN 2: Switch 3 & Signal 2 & Code 2 -->
              {#if colIndex === 2}
                <!-- Switch 3 Lamps -->
                <circle cx="36" cy="40" r="8" fill={switchFieldStatus['3'] === 'Normal' ? 'url(#jewel-green-lit)' : 'url(#jewel-green-dark)'} stroke="#cbd5e1" stroke-width="1.5" filter="url(#chrome-glow)" />
                <circle cx="108" cy="40" r="8" fill={switchFieldStatus['3'] === 'Reverse' ? 'url(#jewel-amber-lit)' : 'url(#jewel-amber-dark)'} stroke="#cbd5e1" stroke-width="1.5" filter="url(#chrome-glow)" />

                <!-- Switch 3 Shield Plate & Lever -->
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <g class="uss-plate-group" role="button" tabindex="0"
                  onclick={() => toggleSwitchLever('3')}
                  oncontextmenu={(e) => cycleDog('switch', '3', e)}
                >
                  <path
                    d="M 50,62 L 94,62 Q 102,62 106,70 L 122,86 Q 128,92 120,102 L 88,144 Q 78,156 72,156 Q 66,156 56,144 L 24,102 Q 16,92 22,86 L 38,70 Q 42,62 50,62 Z"
                    fill="#0a0a0a"
                    stroke="#ffffff"
                    stroke-width="1.8"
                  />
                  <text x="72" y="75" text-anchor="middle" fill="#ffffff" font-size="14" font-weight="900" font-family="'Arial', sans-serif">3</text>
                  <text x="72" y="85" text-anchor="middle" fill="#cbd5e1" font-size="7.5" font-weight="800" letter-spacing="1">SWITCH</text>

                  <text x="36" y="99" text-anchor="middle" fill="#ffffff" font-size="13" font-weight="900" font-family="'Arial', sans-serif">N</text>
                  <text x="108" y="99" text-anchor="middle" fill="#ffffff" font-size="13" font-weight="900" font-family="'Arial', sans-serif">R</text>

                  <g class="uss-lever-rotor" transform="translate(72, 144) rotate({switchDemands['3'] === 'Normal' ? -30 : 30})">
                    <path
                      d="M -7,0 C -10,-14 -8,-36 -3,-46 C -1,-50 1,-50 3,-46 C 8,-36 10,-14 7,0 C 4,6 -4,6 -7,0 Z"
                      fill="url(#paddle-plastic)"
                      stroke="#0f172a"
                      stroke-width="1.5"
                    />
                    <line x1="0" y1="-46" x2="0" y2="-6" stroke="#f8fafc" stroke-width="2.5" stroke-linecap="round" />
                    <circle cx="0" cy="0" r="14" fill="url(#hub-chrome)" stroke="#090d16" stroke-width="1.5" />
                    <circle cx="0" cy="0" r="5" fill="#1e293b" />
                  </g>
                </g>

                <!-- Signal 2 Lamps in 2 Layers -->
                <circle cx="72" cy="206" r="8" fill={signalAspects['2NAB'] === 'Stop' && signalAspects['2SA'] === 'Stop' ? 'url(#jewel-red-lit)' : 'url(#jewel-red-dark)'} stroke="#cbd5e1" stroke-width="1.5" filter="url(#chrome-glow)" />
                <circle cx="36" cy="234" r="8" fill={signalAspects['2NAB'] !== 'Stop' ? 'url(#jewel-green-lit)' : 'url(#jewel-green-dark)'} stroke="#cbd5e1" stroke-width="1.5" filter="url(#chrome-glow)" />
                <circle cx="108" cy="234" r="8" fill={signalAspects['2SA'] !== 'Stop' ? 'url(#jewel-green-lit)' : 'url(#jewel-green-dark)'} stroke="#cbd5e1" stroke-width="1.5" filter="url(#chrome-glow)" />

                <!-- Signal 2 Shield Plate & Lever -->
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <g class="uss-plate-group" role="button" tabindex="0"
                  onclick={() => cycleSignalLever('2')}
                  oncontextmenu={(e) => cycleDog('signal', '2', e)}
                >
                  <path
                    d="M 50,254 L 94,254 Q 102,254 106,262 L 122,278 Q 128,284 120,294 L 88,336 Q 78,348 72,348 Q 66,348 56,336 L 24,294 Q 16,284 22,278 L 38,262 Q 42,254 50,254 Z"
                    fill="#0a0a0a"
                    stroke="#ffffff"
                    stroke-width="1.8"
                  />
                  <text x="72" y="267" text-anchor="middle" fill="#ffffff" font-size="14" font-weight="900" font-family="'Arial', sans-serif">2</text>
                  <text x="72" y="277" text-anchor="middle" fill="#cbd5e1" font-size="7.5" font-weight="800" letter-spacing="1">SIGNAL</text>

                  <text x="36" y="291" text-anchor="middle" fill="#ffffff" font-size="13" font-weight="900" font-family="'Arial', sans-serif">L</text>
                  <text x="72" y="290" text-anchor="middle" fill="#ffffff" font-size="8.5" font-weight="900" font-family="'Arial', sans-serif">STOP</text>
                  <text x="108" y="291" text-anchor="middle" fill="#ffffff" font-size="13" font-weight="900" font-family="'Arial', sans-serif">R</text>

                  <g class="uss-lever-rotor" transform="translate(72, 336) rotate({signalDemands['2'] === 'Left' ? -30 : signalDemands['2'] === 'Right' ? 30 : 0})">
                    <path
                      d="M -7,0 C -10,-14 -8,-36 -3,-46 C -1,-50 1,-50 3,-46 C 8,-36 10,-14 7,0 C 4,6 -4,6 -7,0 Z"
                      fill="url(#paddle-plastic)"
                      stroke="#0f172a"
                      stroke-width="1.5"
                    />
                    <line x1="0" y1="-46" x2="0" y2="-6" stroke="#f8fafc" stroke-width="2.5" stroke-linecap="round" />
                    <circle cx="0" cy="0" r="14" fill="url(#hub-chrome)" stroke="#090d16" stroke-width="1.5" />
                    <circle cx="0" cy="0" r="5" fill="#1e293b" />
                  </g>
                </g>

                <!-- Code Button 2 -->
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <g class="uss-code-button-interactive" role="button" tabindex="0" onclick={() => punchCodeButton(2)} cursor="pointer">
                  <circle cx="72" cy="436" r="21" fill="url(#button-chrome)" stroke="#94a3b8" stroke-width="2" />
                  <circle cx="72" cy="436" r="15" fill="url(#hub-chrome)" stroke="#475569" stroke-width="1" />
                  <text x="72" y="441" text-anchor="middle" fill="#0f172a" font-size="13" font-weight="900" font-family="'Arial', sans-serif">2</text>
                </g>
              {/if}
            </svg>
          </div>
        {/each}
      </div>
    </div>
</div>

<style>
  .ctc-desk-root {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #1c2617;
    color: #cbd5e1;
    user-select: none;
    overflow: hidden;
  }

  /* Top Banner Bar */
  .faceplate-banner {
    background: #141c11;
    border-bottom: 2px solid #2d3b25;
    padding: 8px 24px;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .banner-title {
    font-size: 13px;
    font-weight: 800;
    letter-spacing: 0.8px;
    color: #f8fafc;
  }

  .banner-status {
    font-size: 11px;
    font-family: monospace;
    color: #94a3b8;
  }

  /* UPPER SECTION: US&S Model Board (Black Background) */
  .model-board-section {
    height: 250px;
    flex: none;
    background: #182214;
    border-bottom: 3px solid #000000;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    padding: 6px 0;
  }

  .model-board-frame {
    width: 1008px;
    height: 240px;
    display: flex;
    flex-direction: column;
    align-items: center;
    background: #0a0d14;
    border: 2px solid #2d3b25;
    border-radius: 6px;
    padding: 4px 10px;
    box-shadow: inset 0 2px 4px rgba(255, 255, 255, 0.05), 0 8px 20px rgba(0, 0, 0, 0.7);
  }

  .model-board-svg {
    width: 100%;
    height: 230px;
  }

  .track-block {
    cursor: pointer;
  }

  .track-block:hover line {
    stroke: #38bdf8;
  }

  /* LOWER TIER: Seamless Continuous US&S Style 504 Lever Deck (US&S Olive Green) */
  .lever-deck-section {
    height: 520px;
    flex: none;
    background: #182214;
    border-top: 2px solid #000000;
    display: flex;
    justify-content: center;
    align-items: center;
    padding: 8px 0 16px;
    box-shadow: inset 0 8px 16px rgba(0, 0, 0, 0.8);
  }

  /* Exactly 1008px Wide to Match Model Board Frame (7 columns of 144px) */
  .uss-continuous-console {
    width: 1008px;
    height: 495px;
    display: flex;
    background: linear-gradient(180deg, #334329 0%, #24311d 60%, #1a2415 100%);
    border: 3px solid #4a5d3c;
    border-radius: 8px;
    box-shadow: 0 16px 32px rgba(0, 0, 0, 0.8), inset 0 1px 2px rgba(255, 255, 255, 0.15);
    overflow: hidden;
  }

  /* Each Column is EXACTLY 144px wide (2.0 inches at 72 DPI) */
  .uss-column-bay {
    width: 144px;
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: space-between;
    padding: 10px 0 16px;
    border-right: 1px solid #1a2315;
    position: relative;
    box-sizing: border-box;
  }

  .uss-column-bay:last-child {
    border-right: none;
  }

  .uss-column-svg {
    width: 144px;
    height: 490px;
    display: block;
  }

  .uss-lever-rotor {
    transition: transform 0.22s cubic-bezier(0.34, 1.56, 0.64, 1);
  }

  .uss-plate-group {
    cursor: pointer;
  }

  .uss-code-button-interactive {
    cursor: pointer;
    transition: transform 0.08s ease;
  }

  .uss-code-button-interactive:hover {
    filter: brightness(1.15);
  }

  .uss-code-button-interactive:active {
    transform: scale(0.96);
    filter: brightness(0.9);
  }

  .unused-column {
    opacity: 0.75;
  }

  .text-red {
    color: #ef4444;
  }

  .text-green {
    color: #22c55e;
  }

  .text-amber {
    color: #f59e0b;
  }

  .text-slate {
    color: #94a3b8;
  }

  .font-bold {
    font-weight: 700;
  }
</style>
