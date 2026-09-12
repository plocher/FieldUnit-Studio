<script lang="ts">
  import { onMount } from 'svelte';
  import { studio } from '$lib/state.svelte';

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
  function punchCodeButton(stationCol: number) {
    const swId = stationCol === 1 ? '1' : stationCol === 2 ? '3' : '5';
    const sigId = stationCol === 1 ? '4' : stationCol === 2 ? '2' : null;

    // 1. Process Switch Command with Detector Lock verification
    const demandedPos = switchDemands[swId];
    const island = swId === '1' ? '1T1' : swId === '3' ? '3T1' : '5T1';

    // Detector Lock: If island track is occupied or time locked, switch machine will not move!
    const isLocked = trackOccupancy[island] || (sigId && timeLockSeconds[sigId] > 0);

    if (!isLocked && switchFieldStatus[swId] !== demandedPos) {
      switchFieldStatus = { ...switchFieldStatus, [swId]: 'Moving' };
      // Simulate Tortoise motor travel time (2.0 seconds)
      setTimeout(() => {
        switchFieldStatus = { ...switchFieldStatus, [swId]: demandedPos };
        evaluatePlantRoutes();
      }, 2000);
    }

    // 2. Process Signal Command with Approach Time Locking
    if (sigId) {
      const demandedSig = signalDemands[sigId];
      // If signal was previously permissive and dispatcher forces it to Stop: engage time lock!
      if (demandedSig === 'Stop' && (signalAspects['2NAB'] !== 'Stop' || signalAspects['2SA'] !== 'Stop')) {
        timeLockSeconds = { ...timeLockSeconds, [sigId]: 15 }; // 15s time lock countdown for simulation
      }
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
        SYSTEM: INTERFACE "A" ASYNCHRONOUS CODELINE |
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
            <line x1="50" y1="28" x2="958" y2="28" stroke="#cbd5e1" stroke-width="2" />
            <line x1="50" y1="40" x2="958" y2="40" stroke="#cbd5e1" stroke-width="2" />
            <!-- Industry spur thin line -->
            <path d="M 320,28 L 380,16 L 680,16" fill="none" stroke="#94a3b8" stroke-width="1.5" />
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

          <!-- Derail 5 Stamped ID & Point Indicator Lamp -->
          <text x="365" y="94" fill="#f8fafc" font-size="13" font-weight="900" font-family="sans-serif">5</text>
          <circle cx="390" cy="105" r="7" fill={switchFieldStatus['5'] === 'Normal' ? '#ef4444' : '#ffffff'} stroke="#000000" stroke-width="2" />

          <!-- Signal 4NA Searchlight Head -->
          <line x1="620" y1="105" x2="620" y2="84" stroke="#cbd5e1" stroke-width="2.5" />
          <circle cx="620" cy="84" r="7" fill={signalAspects['4NA'] !== 'Stop' ? 'url(#jewel-green-lit)' : 'url(#jewel-red-lit)'} stroke="#ffffff" stroke-width="1.5" />
          <text x="636" y="88" fill="#e2e8f0" font-size="10" font-weight="800">4NA</text>

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

          <!-- Signal 4SA on MT2 -->
          <line x1="200" y1="145" x2="200" y2="166" stroke="#cbd5e1" stroke-width="2.5" />
          <circle cx="200" cy="166" r="7" fill={signalAspects['4SA'] !== 'Stop' ? 'url(#jewel-green-lit)' : 'url(#jewel-red-lit)'} stroke="#ffffff" stroke-width="1.5" />
          <text x="200" y="184" text-anchor="middle" fill="#e2e8f0" font-size="10" font-weight="800">4SA</text>

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

          <!-- Signal 2NAB on MT2 (Two Searchlight Heads) -->
          <line x1="720" y1="145" x2="720" y2="112" stroke="#cbd5e1" stroke-width="2.5" />
          <circle cx="720" cy="126" r="6" fill={signalAspects['2NAB'] === 'Clear' ? 'url(#jewel-green-lit)' : 'url(#jewel-red-lit)'} stroke="#ffffff" stroke-width="1.5" />
          <circle cx="720" cy="112" r="6" fill={signalAspects['2NAB'] === 'Diverging' ? 'url(#jewel-green-lit)' : 'url(#jewel-red-lit)'} stroke="#ffffff" stroke-width="1.5" />
          <text x="736" y="120" fill="#e2e8f0" font-size="10" font-weight="800">2NAB</text>

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

          <!-- Signal 2SA Dwarf Searchlight on MT1 -->
          <line x1="200" y1="205" x2="200" y2="224" stroke="#cbd5e1" stroke-width="2.5" />
          <circle cx="200" cy="224" r="6" fill={signalAspects['2SA'] !== 'Stop' ? 'url(#jewel-green-lit)' : 'url(#jewel-red-lit)'} stroke="#ffffff" stroke-width="1.5" />
          <text x="216" y="228" fill="#e2e8f0" font-size="10" font-weight="800">2SA</text>

          <!-- ROW 4: Milepost Numbers Row at Bottom of Model Board -->
          <g class="milepost-row" fill="#64748b" font-size="10" font-weight="800" font-family="monospace">
            <text x="120" y="250" text-anchor="middle">MP 82.5</text>
            <text x="360" y="250" text-anchor="middle">MP 83.0</text>
            <text x="504" y="250" text-anchor="middle">MP 83.2</text>
            <text x="680" y="250" text-anchor="middle">MP 83.5</text>
            <text x="830" y="250" text-anchor="middle">MP 84.0</text>
          </g>
        </svg>
      </div>
    </div>

    <!-- LOWER SECTION: Seamless Continuous US&S Style 504 Lever Deck (Olive Green, Same Width as Model Board) -->
    <div class="lever-deck-section">
      <div class="uss-continuous-console">
        <!-- COLUMN 0: Unused Column with Pre-Punched Empty Holes -->
        <div class="uss-column-bay unused-column">
          <svg viewBox="0 0 144 480" class="uss-unused-bay-svg">
            <!-- Top Mounting Screw -->
            <circle cx="72" cy="14" r="4.5" fill="#080c12" stroke="#1e2918" stroke-width="1.5" />
            <!-- Switch Lamp Holes (Matching active switch lamps at x=36, x=108, y=34) -->
            <circle cx="36" cy="34" r="8" fill="#080c12" stroke="#1e2918" stroke-width="2" />
            <circle cx="108" cy="34" r="8" fill="#080c12" stroke="#1e2918" stroke-width="2" />
            <!-- Switch Lever Shaft Hole (Matching active lever pivot at x=72, y=144) -->
            <circle cx="72" cy="144" r="14" fill="#080c12" stroke="#1e2918" stroke-width="2" />
            <circle cx="72" cy="144" r="4" fill="#040609" />
            <!-- Mid Mounting Screw -->
            <circle cx="72" cy="180" r="4.5" fill="#080c12" stroke="#1e2918" stroke-width="1.5" />
            <!-- Signal Lamp Holes (Matching active signal lamps: Stop at 72,198; L/R at 36,224 and 108,224) -->
            <circle cx="72" cy="198" r="8" fill="#080c12" stroke="#1e2918" stroke-width="2" />
            <circle cx="36" cy="224" r="8" fill="#080c12" stroke="#1e2918" stroke-width="2" />
            <circle cx="108" cy="224" r="8" fill="#080c12" stroke="#1e2918" stroke-width="2" />
            <!-- Signal Lever Shaft Hole (Matching active signal lever pivot at x=72, y=334) -->
            <circle cx="72" cy="334" r="14" fill="#080c12" stroke="#1e2918" stroke-width="2" />
            <circle cx="72" cy="334" r="4" fill="#040609" />
            <!-- Code Button Hole (Matching active code button at x=72, y=438) -->
            <circle cx="72" cy="438" r="18" fill="#080c12" stroke="#1e2918" stroke-width="2" />
          </svg>
        </div>

        <!-- COLUMN 1: Active Station Column 1 (Switch 1 & Signal 4) -->
        <div class="uss-column-bay">
          <!-- Switch 1 Unit (Lamps: N=Green, R=Yellow/Amber on 2" centers at y=34) -->
          <div class="uss-lever-tier">
            <div class="uss-switch-lamp-cluster">
              <div class="jewel-mount left-mount" title="Normal Correspondence (1NWK) - Green">
                <span class="uss-jewel-lens jewel-green" class:lit={switchFieldStatus['1'] === 'Normal'}></span>
              </div>
              <div class="jewel-mount right-mount" title="Reverse Correspondence (1RWK) - Yellow">
                <span class="uss-jewel-lens jewel-amber" class:lit={switchFieldStatus['1'] === 'Reverse'}></span>
              </div>
            </div>

            <!-- US&S Shield Plate & Lever (Pivoting at y=144, matching punched hole) -->
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="uss-lever-shield-container"
              onclick={() => toggleSwitchLever('1')}
              oncontextmenu={(e) => cycleDog('switch', '1', e)}
              title="Switch 1: Click lever to throw (Normal 30° Left ↔ Reverse 30° Right) | Right-click to dog"
            >
              <svg viewBox="0 0 144 116" class="uss-shield-svg">
                <!-- US&S Shield Plate: Triangle with Pot Lid, bottom vertex at (72, 98) around lever hole -->
                <path
                  d="M 50,6 L 94,6 Q 102,6 106,14 L 122,30 Q 128,36 120,46 L 88,94 Q 78,106 72,106 Q 66,106 56,94 L 24,46 Q 16,36 22,30 L 38,14 Q 42,6 50,6 Z"
                  fill="url(#shield-plate-metal)"
                  stroke="#94a3b8"
                  stroke-width="1.5"
                />
                <!-- Center Stamped Number & Type Label in Raised Pot Lid -->
                <text x="72" y="21" text-anchor="middle" fill="#ffffff" font-size="14" font-weight="900" font-family="'Arial', sans-serif">1</text>
                <text x="72" y="30" text-anchor="middle" fill="#94a3b8" font-size="7.5" font-weight="800" letter-spacing="1">SWITCH</text>

                <!-- Unanimated Large Bold White Letters on Shoulders Directly Under Lamps -->
                <text x="36" y="44" text-anchor="middle" fill="#ffffff" font-size="13" font-weight="900" font-family="'Arial', sans-serif">N</text>
                <text x="108" y="44" text-anchor="middle" fill="#ffffff" font-size="13" font-weight="900" font-family="'Arial', sans-serif">R</text>

                <!-- Lever Assembly (Pivots at x=72, y=94 - tip stays below number/words!) -->
                <g class="uss-lever-rotor" transform="translate(72, 94) rotate({switchDemands['1'] === 'Normal' ? -30 : 30})">
                  <!-- Teardrop Paddle Blade -->
                  <path
                    d="M -7,0 C -10,-14 -8,-36 -3,-46 C -1,-50 1,-50 3,-46 C 8,-36 10,-14 7,0 C 4,6 -4,6 -7,0 Z"
                    fill="url(#paddle-plastic)"
                    stroke="#0f172a"
                    stroke-width="1.5"
                  />
                  <!-- White Molded Pointer Stripe -->
                  <line x1="0" y1="-46" x2="0" y2="-6" stroke="#f8fafc" stroke-width="2.5" stroke-linecap="round" />
                  <!-- Center Chrome Hub -->
                  <circle cx="0" cy="0" r="14" fill="url(#hub-chrome)" stroke="#090d16" stroke-width="1.5" />
                  <circle cx="0" cy="0" r="5" fill="#1e293b" />
                </g>
              </svg>

              {#if switchDogs['1'] !== 'none'}
                <div class="uss-dog-badge dog-{switchDogs['1']}">DOG</div>
              {/if}
            </div>
          </div>

          <!-- Signal 4 Unit (2 Layers: Upper Center Red STOP at y=198, Lower Green L & R at y=224) -->
          <div class="uss-lever-tier">
            <div class="uss-signal-lamp-cluster">
              <!-- Upper Layer (Center): Red Stop Indication -->
              <div class="signal-lamp-stop-top" title="Stop Indication - Red">
                <span class="uss-jewel-lens jewel-red" class:lit={signalAspects['4NA'] === 'Stop' && signalAspects['4SA'] === 'Stop'}></span>
              </div>
              <!-- Lower Layer (Left & Right on 2\" centers): Green Permissive Indications -->
              <div class="signal-lamp-row-bottom">
                <div class="jewel-mount left-mount" title="Left Permissive (4NA) - Green">
                  <span class="uss-jewel-lens jewel-green" class:lit={signalAspects['4NA'] !== 'Stop'}></span>
                </div>
                <div class="jewel-mount right-mount" title="Right Permissive (4SA) - Green">
                  <span class="uss-jewel-lens jewel-green" class:lit={signalAspects['4SA'] !== 'Stop'}></span>
                </div>
              </div>
            </div>

            <!-- Signal 4 Shield Plate & Lever (Pivoting at y=334, matching punched hole) -->
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="uss-lever-shield-container"
              onclick={() => cycleSignalLever('4')}
              oncontextmenu={(e) => cycleDog('signal', '4', e)}
              title="Signal 4: Click lever to throw (Left 30° ↔ Stop Center 0° ↔ Right 30°) | Right-click to dog"
            >
              <svg viewBox="0 0 144 116" class="uss-shield-svg">
                <path
                  d="M 50,6 L 94,6 Q 102,6 106,14 L 122,30 Q 128,36 120,46 L 88,94 Q 78,106 72,106 Q 66,106 56,94 L 24,46 Q 16,36 22,30 L 38,14 Q 42,6 50,6 Z"
                  fill="url(#shield-plate-metal)"
                  stroke="#94a3b8"
                  stroke-width="1.5"
                />
                <text x="72" y="21" text-anchor="middle" fill="#ffffff" font-size="14" font-weight="900" font-family="'Arial', sans-serif">4</text>
                <text x="72" y="30" text-anchor="middle" fill="#94a3b8" font-size="7.5" font-weight="800" letter-spacing="1">SIGNAL</text>

                <!-- Unanimated Large Bold White Labels -->
                <text x="36" y="44" text-anchor="middle" fill="#ffffff" font-size="13" font-weight="900" font-family="'Arial', sans-serif">L</text>
                <text x="72" y="42" text-anchor="middle" fill="#ffffff" font-size="8.5" font-weight="900" font-family="'Arial', sans-serif">STOP</text>
                <text x="108" y="44" text-anchor="middle" fill="#ffffff" font-size="13" font-weight="900" font-family="'Arial', sans-serif">R</text>

                <!-- Signal 4 Paddle Lever (Pivoting at x=72, y=94) -->
                <g class="uss-lever-rotor" transform="translate(72, 94) rotate({signalDemands['4'] === 'Left' ? -30 : signalDemands['4'] === 'Right' ? 30 : 0})">
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
              </svg>

              {#if signalDogs['4'] !== 'none'}
                <div class="uss-dog-badge dog-{signalDogs['4']}">DOG</div>
              {/if}
            </div>
          </div>

          <!-- Authentic Round Machined US&S Code Button at y=438 (Never cut off) -->
          <div class="code-button-mount">
            <button class="uss-round-code-button" onclick={() => punchCodeButton(1)} title="Punch to transmit atomic snapshot">
              <div class="round-button-outer-rim">
                <div class="round-button-piston">
                  <span class="button-piston-text">1</span>
                </div>
              </div>
            </button>
          </div>
        </div>

        <!-- COLUMN 2: Active Station Column 2 (Switch 3 & Signal 2) -->
        <div class="uss-column-bay">
          <!-- Switch 3 Unit -->
          <div class="uss-lever-tier">
            <div class="uss-switch-lamp-cluster">
              <div class="jewel-mount left-mount" title="Normal Correspondence (3NWK) - Green">
                <span class="uss-jewel-lens jewel-green" class:lit={switchFieldStatus['3'] === 'Normal'}></span>
              </div>
              <div class="jewel-mount right-mount" title="Reverse Correspondence (3RWK) - Yellow">
                <span class="uss-jewel-lens jewel-amber" class:lit={switchFieldStatus['3'] === 'Reverse'}></span>
              </div>
            </div>

            <!-- Switch 3 Shield Plate & Lever -->
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="uss-lever-shield-container"
              onclick={() => toggleSwitchLever('3')}
              oncontextmenu={(e) => cycleDog('switch', '3', e)}
              title="Switch 3: Click lever to throw (Normal 30° Left ↔ Reverse 30° Right) | Right-click to dog"
            >
              <svg viewBox="0 0 144 116" class="uss-shield-svg">
                <path
                  d="M 50,6 L 94,6 Q 102,6 106,14 L 122,30 Q 128,36 120,46 L 88,94 Q 78,106 72,106 Q 66,106 56,94 L 24,46 Q 16,36 22,30 L 38,14 Q 42,6 50,6 Z"
                  fill="url(#shield-plate-metal)"
                  stroke="#94a3b8"
                  stroke-width="1.5"
                />
                <text x="72" y="21" text-anchor="middle" fill="#ffffff" font-size="14" font-weight="900" font-family="'Arial', sans-serif">3</text>
                <text x="72" y="30" text-anchor="middle" fill="#94a3b8" font-size="7.5" font-weight="800" letter-spacing="1">SWITCH</text>

                <text x="36" y="44" text-anchor="middle" fill="#ffffff" font-size="13" font-weight="900" font-family="'Arial', sans-serif">N</text>
                <text x="108" y="44" text-anchor="middle" fill="#ffffff" font-size="13" font-weight="900" font-family="'Arial', sans-serif">R</text>

                <g class="uss-lever-rotor" transform="translate(72, 94) rotate({switchDemands['3'] === 'Normal' ? -30 : 30})">
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
              </svg>

              {#if switchDogs['3'] !== 'none'}
                <div class="uss-dog-badge dog-{switchDogs['3']}">DOG</div>
              {/if}
            </div>
          </div>

          <!-- Signal 2 Unit -->
          <div class="uss-lever-tier">
            <div class="uss-signal-lamp-cluster">
              <div class="signal-lamp-stop-top" title="Stop Indication - Red">
                <span class="uss-jewel-lens jewel-red" class:lit={signalAspects['2NAB'] === 'Stop' && signalAspects['2SA'] === 'Stop'}></span>
              </div>
              <div class="signal-lamp-row-bottom">
                <div class="jewel-mount left-mount" title="Left Permissive (2NAB) - Green">
                  <span class="uss-jewel-lens jewel-green" class:lit={signalAspects['2NAB'] !== 'Stop'}></span>
                </div>
                <div class="jewel-mount right-mount" title="Right Permissive (2SA) - Green">
                  <span class="uss-jewel-lens jewel-green" class:lit={signalAspects['2SA'] !== 'Stop'}></span>
                </div>
              </div>
            </div>

            <!-- Signal 2 Shield Plate & Lever -->
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="uss-lever-shield-container"
              onclick={() => cycleSignalLever('2')}
              oncontextmenu={(e) => cycleDog('signal', '2', e)}
              title="Signal 2: Click lever to throw (Left 30° ↔ Stop Center 0° ↔ Right 30°) | Right-click to dog"
            >
              <svg viewBox="0 0 144 116" class="uss-shield-svg">
                <path
                  d="M 50,6 L 94,6 Q 102,6 106,14 L 122,30 Q 128,36 120,46 L 88,94 Q 78,106 72,106 Q 66,106 56,94 L 24,46 Q 16,36 22,30 L 38,14 Q 42,6 50,6 Z"
                  fill="url(#shield-plate-metal)"
                  stroke="#94a3b8"
                  stroke-width="1.5"
                />
                <text x="72" y="21" text-anchor="middle" fill="#ffffff" font-size="14" font-weight="900" font-family="'Arial', sans-serif">2</text>
                <text x="72" y="30" text-anchor="middle" fill="#94a3b8" font-size="7.5" font-weight="800" letter-spacing="1">SIGNAL</text>

                <text x="36" y="44" text-anchor="middle" fill="#ffffff" font-size="13" font-weight="900" font-family="'Arial', sans-serif">L</text>
                <text x="72" y="42" text-anchor="middle" fill="#ffffff" font-size="8.5" font-weight="900" font-family="'Arial', sans-serif">STOP</text>
                <text x="108" y="44" text-anchor="middle" fill="#ffffff" font-size="13" font-weight="900" font-family="'Arial', sans-serif">R</text>

                <g class="uss-lever-rotor" transform="translate(72, 94) rotate({signalDemands['2'] === 'Left' ? -30 : signalDemands['2'] === 'Right' ? 30 : 0})">
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
              </svg>

              {#if signalDogs['2'] !== 'none'}
                <div class="uss-dog-badge dog-{signalDogs['2']}">DOG</div>
              {/if}
            </div>
          </div>

          <!-- Code Button 2 -->
          <div class="code-button-mount">
            <button class="uss-round-code-button" onclick={() => punchCodeButton(2)} title="Punch to transmit atomic snapshot">
              <div class="round-button-outer-rim">
                <div class="round-button-piston">
                  <span class="button-piston-text">2</span>
                </div>
              </div>
            </button>
          </div>
        </div>

        <!-- COLUMN 3: Active Station Column 3 (Derail 5 / Electric Lock) -->
        <div class="uss-column-bay">
          <!-- Switch 5 (Electric Lock / Derail) Unit: Green=N=LOCKED / Red=R=UNLOCKED -->
          <div class="uss-lever-tier">
            <div class="uss-switch-lamp-cluster">
              <div class="jewel-mount left-mount" title="Locked (5NWK) - Green">
                <span class="uss-jewel-lens jewel-green" class:lit={switchFieldStatus['5'] === 'Normal'}></span>
              </div>
              <div class="jewel-mount right-mount" title="Unlocked (5RWK) - Red">
                <span class="uss-jewel-lens jewel-red" class:lit={switchFieldStatus['5'] === 'Reverse'}></span>
              </div>
            </div>

            <!-- Lock 5 Shield Plate & Lever -->
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="uss-lever-shield-container"
              onclick={() => toggleSwitchLever('5')}
              oncontextmenu={(e) => cycleDog('switch', '5', e)}
              title="Lock 5: Click lever to throw (Locked 30° Left ↔ Unlocked 30° Right) | Right-click to dog"
            >
              <svg viewBox="0 0 144 116" class="uss-shield-svg">
                <path
                  d="M 50,6 L 94,6 Q 102,6 106,14 L 122,30 Q 128,36 120,46 L 88,94 Q 78,106 72,106 Q 66,106 56,94 L 24,46 Q 16,36 22,30 L 38,14 Q 42,6 50,6 Z"
                  fill="url(#shield-plate-metal)"
                  stroke="#94a3b8"
                  stroke-width="1.5"
                />
                <text x="72" y="21" text-anchor="middle" fill="#ffffff" font-size="14" font-weight="900" font-family="'Arial', sans-serif">5</text>
                <text x="72" y="30" text-anchor="middle" fill="#94a3b8" font-size="7.5" font-weight="800" letter-spacing="1">LOCK</text>

                <text x="36" y="44" text-anchor="middle" fill="#ffffff" font-size="12" font-weight="900" font-family="'Arial', sans-serif">LKD</text>
                <text x="108" y="44" text-anchor="middle" fill="#ffffff" font-size="11" font-weight="900" font-family="'Arial', sans-serif">UNLKD</text>

                <g class="uss-lever-rotor" transform="translate(72, 94) rotate({switchDemands['5'] === 'Normal' ? -30 : 30})">
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
              </svg>

              {#if switchDogs['5'] !== 'none'}
                <div class="uss-dog-badge dog-{switchDogs['5']}">DOG</div>
              {/if}
            </div>
          </div>

          <!-- Blank Lower Section (Matching Signal Holes on Unused Column!) -->
          <div class="uss-lever-tier blank-tier">
            <svg viewBox="0 0 144 160" class="uss-blank-tier-svg">
              <circle cx="72" cy="18" r="8" fill="#080c12" stroke="#1e2918" stroke-width="2" />
              <circle cx="36" cy="44" r="8" fill="#080c12" stroke="#1e2918" stroke-width="2" />
              <circle cx="108" cy="44" r="8" fill="#080c12" stroke="#1e2918" stroke-width="2" />
              <circle cx="72" cy="154" r="14" fill="#080c12" stroke="#1e2918" stroke-width="2" />
              <circle cx="72" cy="154" r="4" fill="#040609" />
            </svg>
          </div>

          <!-- Code Button 3 -->
          <div class="code-button-mount">
            <button class="uss-round-code-button" onclick={() => punchCodeButton(3)} title="Punch to transmit atomic snapshot">
              <div class="round-button-outer-rim">
                <div class="round-button-piston">
                  <span class="button-piston-text">3</span>
                </div>
              </div>
            </button>
          </div>
        </div>

        <!-- COLUMN 4: Unused Column with Pre-Punched Empty Holes -->
        <div class="uss-column-bay unused-column">
          <svg viewBox="0 0 144 480" class="uss-unused-bay-svg">
            <circle cx="72" cy="14" r="4.5" fill="#080c12" stroke="#1e2918" stroke-width="1.5" />
            <circle cx="36" cy="34" r="8" fill="#080c12" stroke="#1e2918" stroke-width="2" />
            <circle cx="108" cy="34" r="8" fill="#080c12" stroke="#1e2918" stroke-width="2" />
            <circle cx="72" cy="144" r="14" fill="#080c12" stroke="#1e2918" stroke-width="2" />
            <circle cx="72" cy="144" r="4" fill="#040609" />
            <circle cx="72" cy="180" r="4.5" fill="#080c12" stroke="#1e2918" stroke-width="1.5" />
            <circle cx="72" cy="198" r="8" fill="#080c12" stroke="#1e2918" stroke-width="2" />
            <circle cx="36" cy="224" r="8" fill="#080c12" stroke="#1e2918" stroke-width="2" />
            <circle cx="108" cy="224" r="8" fill="#080c12" stroke="#1e2918" stroke-width="2" />
            <circle cx="72" cy="334" r="14" fill="#080c12" stroke="#1e2918" stroke-width="2" />
            <circle cx="72" cy="334" r="4" fill="#040609" />
            <circle cx="72" cy="438" r="18" fill="#080c12" stroke="#1e2918" stroke-width="2" />
          </svg>
        </div>

        <!-- COLUMN 5: Unused Column with Pre-Punched Empty Holes -->
        <div class="uss-column-bay unused-column">
          <svg viewBox="0 0 144 480" class="uss-unused-bay-svg">
            <circle cx="72" cy="14" r="4.5" fill="#080c12" stroke="#1e2918" stroke-width="1.5" />
            <circle cx="36" cy="34" r="8" fill="#080c12" stroke="#1e2918" stroke-width="2" />
            <circle cx="108" cy="34" r="8" fill="#080c12" stroke="#1e2918" stroke-width="2" />
            <circle cx="72" cy="144" r="14" fill="#080c12" stroke="#1e2918" stroke-width="2" />
            <circle cx="72" cy="144" r="4" fill="#040609" />
            <circle cx="72" cy="180" r="4.5" fill="#080c12" stroke="#1e2918" stroke-width="1.5" />
            <circle cx="72" cy="198" r="8" fill="#080c12" stroke="#1e2918" stroke-width="2" />
            <circle cx="36" cy="224" r="8" fill="#080c12" stroke="#1e2918" stroke-width="2" />
            <circle cx="108" cy="224" r="8" fill="#080c12" stroke="#1e2918" stroke-width="2" />
            <circle cx="72" cy="334" r="14" fill="#080c12" stroke="#1e2918" stroke-width="2" />
            <circle cx="72" cy="334" r="4" fill="#040609" />
            <circle cx="72" cy="438" r="18" fill="#080c12" stroke="#1e2918" stroke-width="2" />
          </svg>
        </div>

        <!-- COLUMN 6: Unused Column with Pre-Punched Empty Holes -->
        <div class="uss-column-bay unused-column">
          <svg viewBox="0 0 144 480" class="uss-unused-bay-svg">
            <circle cx="72" cy="14" r="4.5" fill="#080c12" stroke="#1e2918" stroke-width="1.5" />
            <circle cx="36" cy="34" r="8" fill="#080c12" stroke="#1e2918" stroke-width="2" />
            <circle cx="108" cy="34" r="8" fill="#080c12" stroke="#1e2918" stroke-width="2" />
            <circle cx="72" cy="144" r="14" fill="#080c12" stroke="#1e2918" stroke-width="2" />
            <circle cx="72" cy="144" r="4" fill="#040609" />
            <circle cx="72" cy="180" r="4.5" fill="#080c12" stroke="#1e2918" stroke-width="1.5" />
            <circle cx="72" cy="198" r="8" fill="#080c12" stroke="#1e2918" stroke-width="2" />
            <circle cx="36" cy="224" r="8" fill="#080c12" stroke="#1e2918" stroke-width="2" />
            <circle cx="108" cy="224" r="8" fill="#080c12" stroke="#1e2918" stroke-width="2" />
            <circle cx="72" cy="334" r="14" fill="#080c12" stroke="#1e2918" stroke-width="2" />
            <circle cx="72" cy="334" r="4" fill="#040609" />
            <circle cx="72" cy="438" r="18" fill="#080c12" stroke="#1e2918" stroke-width="2" />
          </svg>
        </div>
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

  .uss-lever-tier {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    margin-bottom: 2px;
  }

  .blank-tier {
    height: 154px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .blank-indicator {
    font-size: 11px;
    font-weight: 800;
    color: #4a5d3c;
    letter-spacing: 1.5px;
  }

  /* Switch Lamp Cluster */
  .uss-switch-lamp-cluster {
    position: relative;
    width: 144px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  /* Signal Lamps in TWO Layers */
  .uss-signal-lamp-cluster {
    position: relative;
    width: 144px;
    height: 46px;
  }

  .signal-lamp-stop-top {
    position: absolute;
    top: 0;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .signal-lamp-row-bottom {
    position: absolute;
    bottom: 0;
    left: 0;
    width: 144px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .jewel-mount {
    position: absolute;
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  /* Exactly 36px from edge = 72px center-to-center spacing */
  .left-mount {
    left: 28px;
  }

  .right-mount {
    right: 28px;
  }

  .uss-jewel-lens {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 2px solid #cbd5e1;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.8);
    transition: all 0.15s ease;
  }

  .jewel-amber {
    background: radial-gradient(circle at 35% 35%, #78350f, #291004);
  }

  .jewel-amber.lit {
    background: radial-gradient(circle at 35% 35%, #fef08a 0%, #f59e0b 50%, #b45309 100%);
    box-shadow: 0 0 14px #f59e0b, 0 0 28px #d97706;
    border-color: #fef08a;
  }

  .jewel-red {
    background: radial-gradient(circle at 35% 35%, #450a0a, #1c0505);
  }

  .jewel-red.lit {
    background: radial-gradient(circle at 35% 35%, #fca5a5 0%, #ef4444 50%, #991b1b 100%);
    box-shadow: 0 0 14px #ef4444, 0 0 28px #b91c1c;
    border-color: #fca5a5;
  }

  .jewel-green {
    background: radial-gradient(circle at 35% 35%, #052e16, #02150a);
  }

  .jewel-green.lit {
    background: radial-gradient(circle at 35% 35%, #86efac 0%, #22c55e 50%, #15803d 100%);
    box-shadow: 0 0 14px #22c55e, 0 0 28px #15803d;
    border-color: #86efac;
  }

  .jewel-letter {
    font-size: 8px;
    font-weight: 800;
    color: #94a3b8;
    margin-top: 1px;
  }

  /* US&S Shield Plate & Lever Container */
  .uss-lever-shield-container {
    position: relative;
    width: 144px;
    height: 116px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }

  .uss-shield-svg {
    width: 144px;
    height: 116px;
    overflow: visible;
  }

  .uss-lever-rotor {
    transition: transform 0.22s cubic-bezier(0.34, 1.56, 0.64, 1);
  }

  /* Pre-Punched Empty Holes on Unused Columns and Blank Tiers */
  .uss-unused-bay-svg {
    width: 144px;
    height: 480px;
    opacity: 0.6;
  }

  .uss-blank-tier-svg {
    width: 144px;
    height: 160px;
    opacity: 0.6;
  }

  /* Code Button Mount: Never cut off */
  .code-button-mount {
    margin-top: auto;
    padding-bottom: 8px;
    flex-shrink: 0;
  }

  .uss-round-code-button {
    width: 44px;
    height: 44px;
    background: transparent;
    border: none;
    padding: 0;
    cursor: pointer;
  }

  .round-button-outer-rim {
    width: 44px;
    height: 44px;
    border-radius: 50%;
    background: linear-gradient(135deg, #cbd5e1 0%, #475569 50%, #0f172a 100%);
    border: 2px solid #94a3b8;
    padding: 4px;
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.7), inset 0 1px 2px rgba(255, 255, 255, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.1s ease;
  }

  .uss-round-code-button:hover .round-button-outer-rim {
    filter: brightness(1.2);
  }

  .uss-round-code-button:active .round-button-outer-rim {
    transform: scale(0.95);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.8);
  }

  .round-button-piston {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: radial-gradient(circle at 35% 35%, #ffffff 0%, #cbd5e1 50%, #64748b 100%);
    border: 1px solid #475569;
    box-shadow: inset 0 2px 4px rgba(255, 255, 255, 0.8), 0 1px 2px rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .button-piston-text {
    font-size: 13px;
    font-weight: 900;
    color: #0f172a;
  }

  /* Pre-Punched Empty Holes on Unused Machine Columns */
  .unused-column {
    opacity: 0.6;
    justify-content: flex-start;
    gap: 16px;
    padding-top: 18px;
  }

  .hole-screw-top,
  .hole-screw-mid {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #090e15;
    border: 1.5px solid #1a2416;
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.8);
  }

  .hole-lamps-switch {
    position: relative;
    width: 144px;
    height: 24px;
    display: flex;
    justify-content: space-between;
    padding: 0 28px;
    box-sizing: border-box;
  }

  .hole-lamps-signal {
    position: relative;
    width: 144px;
    height: 42px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: space-between;
  }

  .punched-hole-center {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: #080c12;
    border: 1.5px solid #1e2918;
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.9);
  }

  .punched-hole-row {
    width: 144px;
    display: flex;
    justify-content: space-between;
    padding: 0 28px;
    box-sizing: border-box;
  }

  .punched-hole {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: #080c12;
    border: 1.5px solid #1e2918;
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.9);
  }

  .punched-hole-large {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: #080c12;
    border: 2px solid #1e2918;
    box-shadow: inset 0 3px 6px rgba(0, 0, 0, 0.95);
  }

  .punched-hole-button {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: #080c12;
    border: 2px solid #1e2918;
    box-shadow: inset 0 3px 6px rgba(0, 0, 0, 0.95);
  }

  .hole-lever-switch,
  .hole-lever-signal {
    height: 80px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .hole-code-button {
    margin-top: auto;
    padding-bottom: 12px;
  }

  /* Mechanical Blocking Dogs */
  .uss-dog-badge {
    position: absolute;
    top: 0;
    right: -4px;
    font-size: 8px;
    font-weight: 900;
    padding: 2px 4px;
    border-radius: 3px;
    color: #ffffff;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.8);
    z-index: 10;
  }

  .dog-red {
    background: #dc2626;
    border: 1px solid #fca5a5;
  }

  .dog-blue {
    background: #2563eb;
    border: 1px solid #93c5fd;
  }

  .text-red {
    color: #ef4444;
  }

  .text-green {
    color: #22c55e;
  }

  .font-bold {
    font-weight: 700;
  }
</style>
