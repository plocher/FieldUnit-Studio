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

  // Toggle switch lever (US&S 60° detent throw)
  function toggleSwitchLever(swId: string) {
    if (switchDogs[swId] !== 'none') return;
    const next = switchDemands[swId] === 'Normal' ? 'Reverse' : 'Normal';
    switchDemands = { ...switchDemands, [swId]: next };
  }

  // Cycle signal lever (US&S 45° detents: Left -> Stop -> Right -> Stop)
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
  <!-- Top Faceplate Header -->
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
      <div class="station-banner">
        <span class="station-callout">CP CORPORAL</span>
        <span class="station-mp">M.P. 83.2</span>
      </div>

      <svg class="model-board-svg" viewBox="0 0 960 220">
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

          <!-- Molded Bakelite Paddle Handle Gradient -->
          <linearGradient id="paddle-plastic" x1="0%" y1="0%" x2="100%" y2="0%">
            <stop offset="0%" stop-color="#1e293b" />
            <stop offset="50%" stop-color="#475569" />
            <stop offset="100%" stop-color="#0f172a" />
          </linearGradient>

          <!-- Machined Chrome Center Pivot Hub Gradient -->
          <radialGradient id="hub-chrome" cx="35%" cy="35%" r="65%">
            <stop offset="0%" stop-color="#ffffff" />
            <stop offset="40%" stop-color="#cbd5e1" />
            <stop offset="80%" stop-color="#475569" />
            <stop offset="100%" stop-color="#0f172a" />
          </radialGradient>
        </defs>

        <!-- Board Surface Background -->
        <rect width="100%" height="100%" fill="#0a0d14" rx="4" />

        <!-- Vertical Station Column Guidelines -->
        <line x1="300" y1="20" x2="300" y2="210" stroke="#1e293b" stroke-width="1.5" stroke-dasharray="4 6" />
        <line x1="560" y1="20" x2="560" y2="210" stroke="#1e293b" stroke-width="1.5" stroke-dasharray="4 6" />
        <line x1="740" y1="20" x2="740" y2="210" stroke="#1e293b" stroke-width="1.5" stroke-dasharray="4 6" />

        <!-- Column Alignment Reference Text -->
        <text x="300" y="24" text-anchor="middle" fill="#64748b" font-size="10" font-weight="700" letter-spacing="0.5">LEVER 1 / 4</text>
        <text x="560" y="24" text-anchor="middle" fill="#64748b" font-size="10" font-weight="700" letter-spacing="0.5">LEVER 3 / 2</text>
        <text x="740" y="24" text-anchor="middle" fill="#64748b" font-size="10" font-weight="700" letter-spacing="0.5">LEVER 5</text>

        <!-- TRACK LEVEL -1: Industry Lead (Beet Loaders & Derail 5) -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <g class="track-block" onclick={() => toggleShunt('5T1')}>
          <!-- Bold White Track Line (5px width) -->
          <line x1="420" y1="55" x2="620" y2="55" stroke="#f1f5f9" stroke-width="5" stroke-linecap="round" />
          <!-- Track Occupancy Red Jewel (Embedded in track line) -->
          <circle cx="520" cy="55" r="7" fill={trackOccupancy['5T1'] ? 'url(#jewel-red-lit)' : 'url(#jewel-red-dark)'} stroke="#94a3b8" stroke-width="1.5" filter="url(#chrome-glow)" />
          <text x="520" y="42" text-anchor="middle" fill={trackOccupancy['5T1'] ? '#ef4444' : '#94a3b8'} font-size="9" font-family="monospace" font-weight="700">5T1</text>
        </g>

        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <g class="track-block" onclick={() => toggleShunt('IND1')}>
          <line x1="620" y1="55" x2="840" y2="55" stroke="#f1f5f9" stroke-width="5" stroke-linecap="round" />
          <!-- Red Bumping Post -->
          <rect x="840" y="45" width="6" height="20" fill="#ef4444" stroke="#ffffff" stroke-width="1.5" rx="1" />
          <circle cx="730" cy="55" r="7" fill={trackOccupancy['IND1'] ? 'url(#jewel-red-lit)' : 'url(#jewel-red-dark)'} stroke="#94a3b8" stroke-width="1.5" filter="url(#chrome-glow)" />
          <text x="730" y="42" text-anchor="middle" fill="#94a3b8" font-size="9" font-family="monospace">IND 1</text>
        </g>

        <!-- Derail 5 Stamped ID & Route Indicator -->
        <text x="400" y="44" fill="#f8fafc" font-size="11" font-weight="900" font-family="sans-serif">5</text>
        <!-- Derail Block Pipe -->
        <circle cx="420" cy="55" r="6" fill={switchFieldStatus['5'] === 'Normal' ? '#ef4444' : '#ffffff'} stroke="#000000" stroke-width="1.5" />

        <!-- Signal 4NA Mast & Searchlight Head on Level -1 -->
        <line x1="620" y1="55" x2="620" y2="38" stroke="#cbd5e1" stroke-width="2" />
        <circle cx="620" cy="38" r="6" fill={signalAspects['4NA'] !== 'Stop' ? 'url(#jewel-green-lit)' : 'url(#jewel-red-lit)'} stroke="#ffffff" stroke-width="1.5" />
        <text x="634" y="42" fill="#e2e8f0" font-size="9" font-weight="800">4NA</text>

        <!-- TRACK LEVEL 0: Mainline MT2 (Northbound / Eastward) -->
        <text x="60" y="112" fill="#94a3b8" font-size="10" font-weight="800" letter-spacing="0.5">MAIN 2</text>

        <!-- Approach Block 2SAT -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <g class="track-block" onclick={() => toggleShunt('2SAT')}>
          <line x1="120" y1="125" x2="200" y2="125" stroke="#f1f5f9" stroke-width="5" stroke-linecap="round" />
          <circle cx="160" cy="125" r="7" fill={trackOccupancy['2SAT'] ? 'url(#jewel-red-lit)' : 'url(#jewel-red-dark)'} stroke="#94a3b8" stroke-width="1.5" filter="url(#chrome-glow)" />
          <text x="160" y="112" text-anchor="middle" fill={trackOccupancy['2SAT'] ? '#ef4444' : '#94a3b8'} font-size="9" font-family="monospace">2SAT</text>
        </g>

        <!-- Signal 4SA Mast & Searchlight on MT2 -->
        <line x1="200" y1="125" x2="200" y2="142" stroke="#cbd5e1" stroke-width="2" />
        <circle cx="200" cy="142" r="6" fill={signalAspects['4SA'] !== 'Stop' ? 'url(#jewel-green-lit)' : 'url(#jewel-red-lit)'} stroke="#ffffff" stroke-width="1.5" />
        <text x="200" y="158" text-anchor="middle" fill="#e2e8f0" font-size="9" font-weight="800">4SA</text>

        <!-- Island Block 1T1 across Switch 1 -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <g class="track-block" onclick={() => toggleShunt('1T1')}>
          <line x1="200" y1="125" x2="300" y2="125" stroke="#f1f5f9" stroke-width="5" />
          <line x1="300" y1="125" x2="560" y2="125" stroke="#f1f5f9" stroke-width="5" />
          <!-- Switch 1 Reverse Diverging Crossover Up to Derail 5 -->
          <line x1="300" y1="125" x2="420" y2="55" stroke="#f1f5f9" stroke-width="5" />
          <circle cx="250" cy="125" r="7" fill={trackOccupancy['1T1'] ? 'url(#jewel-red-lit)' : 'url(#jewel-red-dark)'} stroke="#94a3b8" stroke-width="1.5" filter="url(#chrome-glow)" />
          <text x="250" y="112" text-anchor="middle" fill={trackOccupancy['1T1'] ? '#ef4444' : '#94a3b8'} font-size="9" font-family="monospace">1T1</text>
        </g>

        <!-- Stamped Switch 1 Number & Model Board Point Lamps -->
        <text x="290" y="112" fill="#f8fafc" font-size="11" font-weight="900" font-family="sans-serif">1</text>
        <!-- Normal Alignment Lamp (lit when SW1 Normal) -->
        <circle cx="316" cy="125" r="4.5" fill={switchFieldStatus['1'] === 'Normal' ? '#ffffff' : '#1e293b'} stroke="#64748b" stroke-width="1" />
        <!-- Reverse Alignment Lamp (lit when SW1 Reverse) -->
        <circle cx="316" cy="115" r="4.5" fill={switchFieldStatus['1'] === 'Reverse' ? '#f59e0b' : '#1e293b'} stroke="#64748b" stroke-width="1" />

        <!-- Island Block 3T1 across Switch 3 -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <g class="track-block" onclick={() => toggleShunt('3T1')}>
          <line x1="560" y1="125" x2="720" y2="125" stroke="#f1f5f9" stroke-width="5" />
          <!-- Switch 3 Reverse Diagonal Merge Down to MT1 -->
          <line x1="560" y1="125" x2="450" y2="190" stroke="#f1f5f9" stroke-width="5" />
          <circle cx="640" cy="125" r="7" fill={trackOccupancy['3T1'] ? 'url(#jewel-red-lit)' : 'url(#jewel-red-dark)'} stroke="#94a3b8" stroke-width="1.5" filter="url(#chrome-glow)" />
          <text x="640" y="112" text-anchor="middle" fill={trackOccupancy['3T1'] ? '#ef4444' : '#94a3b8'} font-size="9" font-family="monospace">3T1</text>
        </g>

        <!-- Stamped Switch 3 Number & Model Board Point Lamps -->
        <text x="572" y="112" fill="#f8fafc" font-size="11" font-weight="900" font-family="sans-serif">3</text>
        <circle cx="546" cy="125" r="4.5" fill={switchFieldStatus['3'] === 'Normal' ? '#ffffff' : '#1e293b'} stroke="#64748b" stroke-width="1" />
        <circle cx="546" cy="135" r="4.5" fill={switchFieldStatus['3'] === 'Reverse' ? '#f59e0b' : '#1e293b'} stroke="#64748b" stroke-width="1" />

        <!-- Signal 2NAB on MT2 (Two Searchlight Heads) -->
        <line x1="720" y1="125" x2="720" y2="95" stroke="#cbd5e1" stroke-width="2" />
        <circle cx="720" cy="108" r="5.5" fill={signalAspects['2NAB'] === 'Clear' ? 'url(#jewel-green-lit)' : 'url(#jewel-red-lit)'} stroke="#ffffff" stroke-width="1.5" />
        <circle cx="720" cy="95" r="5.5" fill={signalAspects['2NAB'] === 'Diverging' ? 'url(#jewel-green-lit)' : 'url(#jewel-red-lit)'} stroke="#ffffff" stroke-width="1.5" />
        <text x="734" y="103" fill="#e2e8f0" font-size="9" font-weight="800">2NAB</text>

        <!-- Single Track Main Exit 1NAT -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <g class="track-block" onclick={() => toggleShunt('1NAT')}>
          <line x1="720" y1="125" x2="900" y2="125" stroke="#f1f5f9" stroke-width="5" stroke-linecap="round" />
          <circle cx="810" cy="125" r="7" fill={trackOccupancy['1NAT'] ? 'url(#jewel-red-lit)' : 'url(#jewel-red-dark)'} stroke="#94a3b8" stroke-width="1.5" filter="url(#chrome-glow)" />
          <text x="810" y="112" text-anchor="middle" fill={trackOccupancy['1NAT'] ? '#ef4444' : '#94a3b8'} font-size="9" font-family="monospace">1NAT</text>
        </g>

        <!-- TRACK LEVEL 1: Mainline MT1 (Southbound / Westward) -->
        <text x="60" y="178" fill="#94a3b8" font-size="10" font-weight="800" letter-spacing="0.5">MAIN 1</text>

        <!-- Approach Block 1SAT -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <g class="track-block" onclick={() => toggleShunt('1SAT')}>
          <line x1="120" y1="190" x2="200" y2="190" stroke="#f1f5f9" stroke-width="5" stroke-linecap="round" />
          <line x1="200" y1="190" x2="450" y2="190" stroke="#f1f5f9" stroke-width="5" />
          <circle cx="160" cy="190" r="7" fill={trackOccupancy['1SAT'] ? 'url(#jewel-red-lit)' : 'url(#jewel-red-dark)'} stroke="#94a3b8" stroke-width="1.5" filter="url(#chrome-glow)" />
          <text x="160" y="178" text-anchor="middle" fill={trackOccupancy['1SAT'] ? '#ef4444' : '#94a3b8'} font-size="9" font-family="monospace">1SAT</text>
        </g>

        <!-- Signal 2SA Dwarf Searchlight on MT1 -->
        <line x1="200" y1="190" x2="200" y2="205" stroke="#cbd5e1" stroke-width="2" />
        <circle cx="200" cy="205" r="6" fill={signalAspects['2SA'] !== 'Stop' ? 'url(#jewel-green-lit)' : 'url(#jewel-red-lit)'} stroke="#ffffff" stroke-width="1.5" />
        <text x="214" y="210" fill="#e2e8f0" font-size="9" font-weight="800">2SA</text>
      </svg>
    </div>
  </div>

  <!-- LOWER SECTION: The US&S Style 504 Modular Vertical Station Lever Deck -->
  <div class="lever-deck-section">
    <!-- STATION COLUMN 1 (Switch 1 & Signal 4) -->
    <div class="uss-column-plate">
      <!-- Cast Aluminum Station Nameplate Header -->
      <div class="uss-column-header">STATION 1</div>

      <!-- US&S Switch 1 Unit Faceplate -->
      <div class="uss-escutcheon-unit">
        <!-- Switch Jewel Indications: Left=N (White/Opal), Right=R (Amber) -->
        <div class="uss-jewel-header">
          <div class="jewel-cell">
            <span class="uss-jewel-lamp jewel-opal" class:lit={switchFieldStatus['1'] === 'Normal'}></span>
            <span class="jewel-caption">N</span>
          </div>
          <span class="uss-appliance-number">1</span>
          <div class="jewel-cell">
            <span class="uss-jewel-lamp jewel-amber" class:lit={switchFieldStatus['1'] === 'Reverse'}></span>
            <span class="jewel-caption">R</span>
          </div>
        </div>

        <!-- US&S Molded Black Teardrop Paddle Lever with Chrome Pivot Hub -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="uss-lever-bracket"
          onclick={() => toggleSwitchLever('1')}
          oncontextmenu={(e) => cycleDog('switch', '1', e)}
          title="Click lever to throw (Normal ↔ Reverse) | Right-click to dog"
        >
          <!-- SVG Vector-Drawn US&S Teardrop Paddle Handle -->
          <svg
            class="uss-paddle-lever-svg"
            class:lever-reverse={switchDemands['1'] === 'Reverse'}
            viewBox="0 0 60 76"
          >
            <!-- Molded Teardrop Paddle Blade Body (Pivots at x=30, y=52) -->
            <path
              d="M22,52 C20,40 22,22 26,10 C28,5 32,5 34,10 C38,22 40,40 38,52 C36,58 24,58 22,52 Z"
              fill="url(#paddle-plastic)"
              stroke="#0f172a"
              stroke-width="1.5"
            />
            <!-- Raised Grip Serrations / Ridge -->
            <line x1="30" y1="8" x2="30" y2="44" stroke="#ffffff" stroke-width="2" stroke-linecap="round" opacity="0.9" />
            <!-- Heavy Center Pivot Hub with Machined Retaining Ring -->
            <circle cx="30" cy="52" r="14" fill="url(#hub-chrome)" stroke="#090d16" stroke-width="1.5" />
            <circle cx="30" cy="52" r="5" fill="#1e293b" />
          </svg>

          <!-- Mechanical Dog Collar -->
          {#if switchDogs['1'] !== 'none'}
            <div class="uss-dog-collar dog-{switchDogs['1']}">DOG</div>
          {/if}
        </div>

        <!-- Embossed Detent Labels below Hub -->
        <div class="uss-detent-scale">
          <span class="scale-detent" class:active-detent={switchDemands['1'] === 'Normal'}>N</span>
          <span class="scale-detent" class:active-detent={switchDemands['1'] === 'Reverse'}>R</span>
        </div>
      </div>

      <!-- US&S Signal 4 Unit Faceplate -->
      <div class="uss-escutcheon-unit">
        <!-- Signal Jewel Indications: Left=L (Green), Center=STOP (Red), Right=R (Green) -->
        <div class="uss-jewel-header three-jewels">
          <div class="jewel-cell">
            <span class="uss-jewel-lamp jewel-green" class:lit={signalAspects['4NA'] !== 'Stop'}></span>
            <span class="jewel-caption">L</span>
          </div>
          <div class="jewel-cell">
            <span class="uss-jewel-lamp jewel-red" class:lit={signalAspects['4NA'] === 'Stop' && signalAspects['4SA'] === 'Stop'}></span>
            <span class="jewel-caption">STOP</span>
          </div>
          <div class="jewel-cell">
            <span class="uss-jewel-lamp jewel-green" class:lit={signalAspects['4SA'] !== 'Stop'}></span>
            <span class="jewel-caption">R</span>
          </div>
        </div>

        <!-- US&S 3-Position Signal Paddle Lever (-45° Left, 0° Stop, +45° Right) -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="uss-lever-bracket"
          onclick={() => cycleSignalLever('4')}
          oncontextmenu={(e) => cycleDog('signal', '4', e)}
          title="Click lever to throw (Left ↔ Stop ↔ Right) | Right-click to dog"
        >
          <svg
            class="uss-paddle-lever-svg lever-signal-{signalDemands['4'].toLowerCase()}"
            viewBox="0 0 60 76"
          >
            <path
              d="M22,52 C20,40 22,22 26,10 C28,5 32,5 34,10 C38,22 40,40 38,52 C36,58 24,58 22,52 Z"
              fill="url(#paddle-plastic)"
              stroke="#0f172a"
              stroke-width="1.5"
            />
            <line x1="30" y1="8" x2="30" y2="44" stroke="#ffffff" stroke-width="2" stroke-linecap="round" opacity="0.9" />
            <circle cx="30" cy="52" r="14" fill="url(#hub-chrome)" stroke="#090d16" stroke-width="1.5" />
            <circle cx="30" cy="52" r="5" fill="#1e293b" />
          </svg>

          {#if signalDogs['4'] !== 'none'}
            <div class="uss-dog-collar dog-{signalDogs['4']}">DOG</div>
          {/if}
        </div>

        <div class="uss-detent-scale three-detents">
          <span class="scale-detent" class:active-detent={signalDemands['4'] === 'Left'}>L</span>
          <span class="scale-detent" class:active-detent={signalDemands['4'] === 'Stop'}>STOP</span>
          <span class="scale-detent" class:active-detent={signalDemands['4'] === 'Right'}>R</span>
        </div>
      </div>

      <!-- Machined Metal Code Button Head -->
      <button class="uss-code-button-assembly" onclick={() => punchCodeButton(1)} title="Punch to transmit Interface A code line snapshot">
        <div class="code-button-rim">
          <div class="code-button-piston">CODE 1</div>
        </div>
      </button>
    </div>

    <!-- STATION COLUMN 2 (Switch 3 & Signal 2) -->
    <div class="uss-column-plate">
      <div class="uss-column-header">STATION 2</div>

      <!-- US&S Switch 3 Unit Faceplate -->
      <div class="uss-escutcheon-unit">
        <div class="uss-jewel-header">
          <div class="jewel-cell">
            <span class="uss-jewel-lamp jewel-opal" class:lit={switchFieldStatus['3'] === 'Normal'}></span>
            <span class="jewel-caption">N</span>
          </div>
          <span class="uss-appliance-number">3</span>
          <div class="jewel-cell">
            <span class="uss-jewel-lamp jewel-amber" class:lit={switchFieldStatus['3'] === 'Reverse'}></span>
            <span class="jewel-caption">R</span>
          </div>
        </div>

        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="uss-lever-bracket"
          onclick={() => toggleSwitchLever('3')}
          oncontextmenu={(e) => cycleDog('switch', '3', e)}
          title="Click lever to throw (Normal ↔ Reverse) | Right-click to dog"
        >
          <svg
            class="uss-paddle-lever-svg"
            class:lever-reverse={switchDemands['3'] === 'Reverse'}
            viewBox="0 0 60 76"
          >
            <path
              d="M22,52 C20,40 22,22 26,10 C28,5 32,5 34,10 C38,22 40,40 38,52 C36,58 24,58 22,52 Z"
              fill="url(#paddle-plastic)"
              stroke="#0f172a"
              stroke-width="1.5"
            />
            <line x1="30" y1="8" x2="30" y2="44" stroke="#ffffff" stroke-width="2" stroke-linecap="round" opacity="0.9" />
            <circle cx="30" cy="52" r="14" fill="url(#hub-chrome)" stroke="#090d16" stroke-width="1.5" />
            <circle cx="30" cy="52" r="5" fill="#1e293b" />
          </svg>

          {#if switchDogs['3'] !== 'none'}
            <div class="uss-dog-collar dog-{switchDogs['3']}">DOG</div>
          {/if}
        </div>

        <div class="uss-detent-scale">
          <span class="scale-detent" class:active-detent={switchDemands['3'] === 'Normal'}>N</span>
          <span class="scale-detent" class:active-detent={switchDemands['3'] === 'Reverse'}>R</span>
        </div>
      </div>

      <!-- US&S Signal 2 Unit Faceplate -->
      <div class="uss-escutcheon-unit">
        <div class="uss-jewel-header three-jewels">
          <div class="jewel-cell">
            <span class="uss-jewel-lamp jewel-green" class:lit={signalAspects['2NAB'] !== 'Stop'}></span>
            <span class="jewel-caption">L</span>
          </div>
          <div class="jewel-cell">
            <span class="uss-jewel-lamp jewel-red" class:lit={signalAspects['2NAB'] === 'Stop' && signalAspects['2SA'] === 'Stop'}></span>
            <span class="jewel-caption">STOP</span>
          </div>
          <div class="jewel-cell">
            <span class="uss-jewel-lamp jewel-green" class:lit={signalAspects['2SA'] !== 'Stop'}></span>
            <span class="jewel-caption">R</span>
          </div>
        </div>

        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="uss-lever-bracket"
          onclick={() => cycleSignalLever('2')}
          oncontextmenu={(e) => cycleDog('signal', '2', e)}
          title="Click lever to throw (Left ↔ Stop ↔ Right) | Right-click to dog"
        >
          <svg
            class="uss-paddle-lever-svg lever-signal-{signalDemands['2'].toLowerCase()}"
            viewBox="0 0 60 76"
          >
            <path
              d="M22,52 C20,40 22,22 26,10 C28,5 32,5 34,10 C38,22 40,40 38,52 C36,58 24,58 22,52 Z"
              fill="url(#paddle-plastic)"
              stroke="#0f172a"
              stroke-width="1.5"
            />
            <line x1="30" y1="8" x2="30" y2="44" stroke="#ffffff" stroke-width="2" stroke-linecap="round" opacity="0.9" />
            <circle cx="30" cy="52" r="14" fill="url(#hub-chrome)" stroke="#090d16" stroke-width="1.5" />
            <circle cx="30" cy="52" r="5" fill="#1e293b" />
          </svg>

          {#if signalDogs['2'] !== 'none'}
            <div class="uss-dog-collar dog-{signalDogs['2']}">DOG</div>
          {/if}
        </div>

        <div class="uss-detent-scale three-detents">
          <span class="scale-detent" class:active-detent={signalDemands['2'] === 'Left'}>L</span>
          <span class="scale-detent" class:active-detent={signalDemands['2'] === 'Stop'}>STOP</span>
          <span class="scale-detent" class:active-detent={signalDemands['2'] === 'Right'}>R</span>
        </div>
      </div>

      <button class="uss-code-button-assembly" onclick={() => punchCodeButton(2)} title="Punch to transmit Interface A code line snapshot">
        <div class="code-button-rim">
          <div class="code-button-piston">CODE 2</div>
        </div>
      </button>
    </div>

    <!-- STATION COLUMN 3 (Derail 5) -->
    <div class="uss-column-plate">
      <div class="uss-column-header">STATION 3</div>

      <!-- US&S Switch 5 (Derail) Unit Faceplate -->
      <div class="uss-escutcheon-unit">
        <div class="uss-jewel-header">
          <div class="jewel-cell">
            <span class="uss-jewel-lamp jewel-red" class:lit={switchFieldStatus['5'] === 'Normal'}></span>
            <span class="jewel-caption">ON</span>
          </div>
          <span class="uss-appliance-number">5</span>
          <div class="jewel-cell">
            <span class="uss-jewel-lamp jewel-opal" class:lit={switchFieldStatus['5'] === 'Reverse'}></span>
            <span class="jewel-caption">OFF</span>
          </div>
        </div>

        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="uss-lever-bracket"
          onclick={() => toggleSwitchLever('5')}
          oncontextmenu={(e) => cycleDog('switch', '5', e)}
          title="Normal = Derail On | Reverse = Derail Clear"
        >
          <svg
            class="uss-paddle-lever-svg"
            class:lever-reverse={switchDemands['5'] === 'Reverse'}
            viewBox="0 0 60 76"
          >
            <path
              d="M22,52 C20,40 22,22 26,10 C28,5 32,5 34,10 C38,22 40,40 38,52 C36,58 24,58 22,52 Z"
              fill="url(#paddle-plastic)"
              stroke="#0f172a"
              stroke-width="1.5"
            />
            <line x1="30" y1="8" x2="30" y2="44" stroke="#ffffff" stroke-width="2" stroke-linecap="round" opacity="0.9" />
            <circle cx="30" cy="52" r="14" fill="url(#hub-chrome)" stroke="#090d16" stroke-width="1.5" />
            <circle cx="30" cy="52" r="5" fill="#1e293b" />
          </svg>

          {#if switchDogs['5'] !== 'none'}
            <div class="uss-dog-collar dog-{switchDogs['5']}">DOG</div>
          {/if}
        </div>

        <div class="uss-detent-scale">
          <span class="scale-detent" class:active-detent={switchDemands['5'] === 'Normal'}>ON</span>
          <span class="scale-detent" class:active-detent={switchDemands['5'] === 'Reverse'}>OFF</span>
        </div>
      </div>

      <!-- Blank Lower Plate for Station 3 -->
      <div class="uss-escutcheon-unit blank-escutcheon">
        <div class="blank-escutcheon-label">[ BLANK ]</div>
      </div>

      <button class="uss-code-button-assembly" onclick={() => punchCodeButton(3)} title="Punch to transmit Interface A code line snapshot">
        <div class="code-button-rim">
          <div class="code-button-piston">CODE 3</div>
        </div>
      </button>
    </div>
  </div>
</div>

<style>
  .ctc-desk-root {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #080c14;
    color: #cbd5e1;
    user-select: none;
    overflow: hidden;
  }

  /* Top Banner Bar */
  .faceplate-banner {
    background: #0b1120;
    border-bottom: 2px solid #1e293b;
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

  /* UPPER SECTION: US&S Model Board */
  .model-board-section {
    flex: 1;
    background: #06090f;
    border-bottom: 3px solid #000000;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    padding: 12px 20px;
  }

  .model-board-frame {
    width: 100%;
    max-width: 980px;
    display: flex;
    flex-direction: column;
    align-items: center;
    background: #0a0e18;
    border: 2px solid #1e293b;
    border-radius: 6px;
    padding: 10px 14px;
    box-shadow: inset 0 2px 4px rgba(255, 255, 255, 0.05), 0 8px 20px rgba(0, 0, 0, 0.7);
  }

  .station-banner {
    display: flex;
    align-items: baseline;
    gap: 16px;
    margin-bottom: 6px;
  }

  .station-callout {
    font-size: 14px;
    font-weight: 900;
    letter-spacing: 1.5px;
    color: #f8fafc;
    font-family: 'Times New Roman', serif;
  }

  .station-mp {
    font-size: 11px;
    font-weight: 700;
    color: #64748b;
    letter-spacing: 0.5px;
  }

  .model-board-svg {
    width: 100%;
    height: 220px;
  }

  .track-block {
    cursor: pointer;
  }

  .track-block:hover line {
    stroke: #38bdf8;
  }

  /* LOWER TIER: Authentic US&S Modular 2-Inch Vertical Station Lever Deck */
  .lever-deck-section {
    height: 380px;
    background: #0d131f;
    border-top: 2px solid #000000;
    display: flex;
    justify-content: center;
    gap: 32px;
    padding: 16px 20px 20px;
    box-shadow: inset 0 8px 16px rgba(0, 0, 0, 0.6);
  }

  /* Modular Cast Aluminum Column Plate */
  .uss-column-plate {
    width: 190px;
    background: linear-gradient(180deg, #182234 0%, #101726 100%);
    border: 3px solid #334155;
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 10px 12px 14px;
    box-shadow: 0 12px 24px rgba(0, 0, 0, 0.6), inset 0 1px 2px rgba(255, 255, 255, 0.12);
  }

  .uss-column-header {
    font-size: 11px;
    font-weight: 800;
    letter-spacing: 0.8px;
    color: #f1f5f9;
    background: #090d16;
    border: 1px solid #334155;
    border-radius: 4px;
    padding: 3px 18px;
    margin-bottom: 8px;
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.6);
  }

  .uss-escutcheon-unit {
    width: 100%;
    background: #0d1422;
    border: 1px solid #1e293b;
    border-radius: 6px;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 8px 6px;
    margin-bottom: 8px;
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.5);
  }

  .uss-jewel-header {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 20px;
    width: 100%;
    margin-bottom: 2px;
  }

  .uss-jewel-header.three-jewels {
    gap: 12px;
  }

  .jewel-cell {
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  /* Authentic Faceted Glass Jewel Lamp with Raised Chrome Bezel Ring */
  .uss-jewel-lamp {
    width: 15px;
    height: 15px;
    border-radius: 50%;
    border: 2px solid #cbd5e1;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.7);
    transition: all 0.15s ease;
  }

  .jewel-opal {
    background: radial-gradient(circle at 35% 35%, #475569, #1e293b);
  }

  .jewel-opal.lit {
    background: radial-gradient(circle at 35% 35%, #ffffff 0%, #f1f5f9 40%, #cbd5e1 80%);
    box-shadow: 0 0 12px #ffffff, 0 0 24px #e0f2fe;
    border-color: #ffffff;
  }

  .jewel-amber {
    background: radial-gradient(circle at 35% 35%, #78350f, #291004);
  }

  .jewel-amber.lit {
    background: radial-gradient(circle at 35% 35%, #fef08a 0%, #f59e0b 50%, #b45309 100%);
    box-shadow: 0 0 12px #f59e0b, 0 0 24px #d97706;
    border-color: #fef08a;
  }

  .jewel-red {
    background: radial-gradient(circle at 35% 35%, #450a0a, #1c0505);
  }

  .jewel-red.lit {
    background: radial-gradient(circle at 35% 35%, #fca5a5 0%, #ef4444 50%, #991b1b 100%);
    box-shadow: 0 0 12px #ef4444, 0 0 24px #b91c1c;
    border-color: #fca5a5;
  }

  .jewel-green {
    background: radial-gradient(circle at 35% 35%, #052e16, #02150a);
  }

  .jewel-green.lit {
    background: radial-gradient(circle at 35% 35%, #86efac 0%, #22c55e 50%, #15803d 100%);
    box-shadow: 0 0 12px #22c55e, 0 0 24px #15803d;
    border-color: #86efac;
  }

  .jewel-caption {
    font-size: 8px;
    font-weight: 800;
    color: #64748b;
    margin-top: 1px;
  }

  .uss-appliance-number {
    font-size: 14px;
    font-weight: 900;
    color: #f1f5f9;
    font-family: 'Arial', sans-serif;
  }

  /* Lever Bracket and Authentic US&S Molded Teardrop Paddle */
  .uss-lever-bracket {
    position: relative;
    width: 68px;
    height: 74px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }

  .uss-paddle-lever-svg {
    width: 60px;
    height: 74px;
    transform-origin: 30px 52px;
    transition: transform 0.22s cubic-bezier(0.34, 1.56, 0.64, 1);
  }

  /* Switch Lever Detents: 0° Normal (Up), 60° Reverse (Right/Down) */
  .uss-paddle-lever-svg.lever-reverse {
    transform: rotate(60deg);
  }

  /* Signal Lever Detents: -45° Left, 0° Stop, +45° Right */
  .uss-paddle-lever-svg.lever-signal-left {
    transform: rotate(-45deg);
  }

  .uss-paddle-lever-svg.lever-signal-stop {
    transform: rotate(0deg);
  }

  .uss-paddle-lever-svg.lever-signal-right {
    transform: rotate(45deg);
  }

  /* Stamped Detent Scale below lever hub */
  .uss-detent-scale {
    display: flex;
    justify-content: space-between;
    width: 64px;
    font-size: 10px;
    font-weight: 800;
    color: #64748b;
    margin-top: 1px;
  }

  .uss-detent-scale.three-detents {
    width: 76px;
  }

  .scale-detent {
    transition: color 0.15s ease;
  }

  .scale-detent.active-detent {
    color: #38bdf8;
    text-shadow: 0 0 6px rgba(56, 189, 248, 0.6);
  }

  /* Machined US&S Code Button at Bottom */
  .uss-code-button-assembly {
    margin-top: auto;
    width: 120px;
    height: 38px;
    background: transparent;
    border: none;
    padding: 0;
    cursor: pointer;
  }

  .code-button-rim {
    width: 100%;
    height: 100%;
    border-radius: 6px;
    background: linear-gradient(180deg, #64748b, #1e293b);
    border: 2px solid #94a3b8;
    padding: 3px;
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.6), inset 0 1px 2px rgba(255, 255, 255, 0.4);
    transition: all 0.1s ease;
  }

  .uss-code-button-assembly:hover .code-button-rim {
    filter: brightness(1.15);
  }

  .uss-code-button-assembly:active .code-button-rim {
    transform: translateY(2px);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.7);
  }

  .code-button-piston {
    width: 100%;
    height: 100%;
    border-radius: 4px;
    background: linear-gradient(180deg, #e2e8f0 0%, #cbd5e1 50%, #94a3b8 100%);
    border: 1px solid #ffffff;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    font-weight: 900;
    color: #0f172a;
    letter-spacing: 0.8px;
    box-shadow: inset 0 1px 2px rgba(255, 255, 255, 0.8);
  }

  /* Blank Escutcheon Unit */
  .blank-escutcheon {
    height: 120px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .blank-escutcheon-label {
    font-size: 10px;
    font-weight: 700;
    color: #334155;
    letter-spacing: 1px;
  }

  /* Blocking Dogs */
  .uss-dog-collar {
    position: absolute;
    top: -4px;
    right: -8px;
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
