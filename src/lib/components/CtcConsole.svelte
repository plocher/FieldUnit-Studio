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

  <!-- UPPER SECTION: The Model Board -->
  <div class="model-board-section">
    <svg class="model-board-svg" viewBox="0 0 960 300">
      <!-- Dark matte board background -->
      <rect width="100%" height="100%" fill="#0a0e17" />

      <!-- Station Column Centerlines (aligned with lever plates below) -->
      <line x1="360" y1="20" x2="360" y2="280" stroke="#1e293b" stroke-width="1.5" stroke-dasharray="4 6" />
      <line x1="580" y1="20" x2="580" y2="280" stroke="#1e293b" stroke-width="1.5" stroke-dasharray="4 6" />
      <line x1="460" y1="20" x2="460" y2="120" stroke="#1e293b" stroke-width="1.5" stroke-dasharray="4 6" />

      <!-- Column ID headers on model board -->
      <text x="360" y="35" text-anchor="middle" fill="#64748b" font-size="10" font-weight="700">COL 1: SW 1 / SIG 4</text>
      <text x="580" y="35" text-anchor="middle" fill="#64748b" font-size="10" font-weight="700">COL 2: SW 3 / SIG 2</text>
      <text x="460" y="20" text-anchor="middle" fill="#64748b" font-size="9" font-weight="700">COL 3: DERAIL 5</text>

      <!-- TRACK LEVEL -1: Industry Lead (Beet Loaders & Derail 5) -->
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <g class="track-block" onclick={() => toggleShunt('5T1')}>
        <line x1="460" y1="70" x2="600" y2="70" stroke="#e2e8f0" stroke-width="3" />
        <circle cx="530" cy="70" r="4.5" fill={trackOccupancy['5T1'] ? '#ef4444' : '#1e293b'} stroke="#cbd5e1" stroke-width="1" />
        <text x="530" y="60" text-anchor="middle" fill={trackOccupancy['5T1'] ? '#ef4444' : '#94a3b8'} font-size="9" font-family="monospace">5T1</text>
      </g>
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <g class="track-block" onclick={() => toggleShunt('IND1')}>
        <line x1="600" y1="70" x2="820" y2="70" stroke="#e2e8f0" stroke-width="3" />
        <rect x="820" y="62" width="5" height="16" fill="#ef4444" stroke="#ffffff" stroke-width="1" />
        <circle cx="710" cy="70" r="4.5" fill={trackOccupancy['IND1'] ? '#ef4444' : '#1e293b'} stroke="#cbd5e1" stroke-width="1" />
        <text x="710" y="60" text-anchor="middle" fill="#94a3b8" font-size="9" font-family="monospace">IND1</text>
      </g>
      <!-- Signal 4NA on Level -1 -->
      <circle cx="600" cy="52" r="4" fill={signalAspects['4NA'] !== 'Stop' ? '#22c55e' : '#ef4444'} stroke="#ffffff" stroke-width="1" />
      <text x="600" y="44" text-anchor="middle" fill="#e2e8f0" font-size="9" font-weight="700">4NA</text>

      <!-- TRACK LEVEL 0: Mainline MT2 (Northbound) -->
      <!-- Approach 2SAT -->
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <g class="track-block" onclick={() => toggleShunt('2SAT')}>
        <line x1="80" y1="160" x2="220" y2="160" stroke="#e2e8f0" stroke-width="3" />
        <circle cx="150" cy="160" r="4.5" fill={trackOccupancy['2SAT'] ? '#ef4444' : '#1e293b'} stroke="#cbd5e1" stroke-width="1" />
        <text x="150" y="150" text-anchor="middle" fill={trackOccupancy['2SAT'] ? '#ef4444' : '#94a3b8'} font-size="9" font-family="monospace">2SAT</text>
      </g>
      <!-- Signal 4SA on MT2 -->
      <circle cx="220" cy="178" r="4" fill={signalAspects['4SA'] !== 'Stop' ? '#22c55e' : '#ef4444'} stroke="#ffffff" stroke-width="1" />
      <text x="220" y="196" text-anchor="middle" fill="#e2e8f0" font-size="9" font-weight="700">4SA</text>

      <!-- Island 1T1 across Switch 1 -->
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <g class="track-block" onclick={() => toggleShunt('1T1')}>
        <line x1="220" y1="160" x2="360" y2="160" stroke="#e2e8f0" stroke-width="3" />
        <line x1="360" y1="160" x2="580" y2="160" stroke="#e2e8f0" stroke-width="3" />
        <!-- Switch 1 Reverse diverge up to Derail 5 -->
        <line x1="360" y1="160" x2="460" y2="70" stroke="#e2e8f0" stroke-width="3" />
        <circle cx="290" cy="160" r="4.5" fill={trackOccupancy['1T1'] ? '#ef4444' : '#1e293b'} stroke="#cbd5e1" stroke-width="1" />
        <text x="290" y="150" text-anchor="middle" fill={trackOccupancy['1T1'] ? '#ef4444' : '#94a3b8'} font-size="9" font-family="monospace">1T1</text>
      </g>

      <!-- Island 3T1 across Switch 3 -->
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <g class="track-block" onclick={() => toggleShunt('3T1')}>
        <line x1="580" y1="160" x2="740" y2="160" stroke="#e2e8f0" stroke-width="3" />
        <!-- Switch 3 Reverse diagonal crossover merge down to MT1 -->
        <line x1="580" y1="160" x2="480" y2="250" stroke="#e2e8f0" stroke-width="3" />
        <circle cx="660" cy="160" r="4.5" fill={trackOccupancy['3T1'] ? '#ef4444' : '#1e293b'} stroke="#cbd5e1" stroke-width="1" />
        <text x="660" y="150" text-anchor="middle" fill={trackOccupancy['3T1'] ? '#ef4444' : '#94a3b8'} font-size="9" font-family="monospace">3T1</text>
      </g>

      <!-- Signal 2NAB on MT2 (Two Heads: Top=Main, Lower=Diverging) -->
      <circle cx="740" cy="144" r="4" fill={signalAspects['2NAB'] === 'Clear' ? '#22c55e' : '#ef4444'} stroke="#ffffff" stroke-width="1" />
      <circle cx="740" cy="134" r="4" fill={signalAspects['2NAB'] === 'Diverging' ? '#22c55e' : '#ef4444'} stroke="#ffffff" stroke-width="1" />
      <text x="740" y="124" text-anchor="middle" fill="#e2e8f0" font-size="9" font-weight="700">2NAB</text>

      <!-- Single Track East Exit 1NAT -->
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <g class="track-block" onclick={() => toggleShunt('1NAT')}>
        <line x1="740" y1="160" x2="900" y2="160" stroke="#e2e8f0" stroke-width="3" />
        <circle cx="820" cy="160" r="4.5" fill={trackOccupancy['1NAT'] ? '#ef4444' : '#1e293b'} stroke="#cbd5e1" stroke-width="1" />
        <text x="820" y="150" text-anchor="middle" fill={trackOccupancy['1NAT'] ? '#ef4444' : '#94a3b8'} font-size="9" font-family="monospace">1NAT</text>
      </g>

      <!-- TRACK LEVEL 1: Mainline MT1 (Southbound) -->
      <!-- Approach 1SAT -->
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <g class="track-block" onclick={() => toggleShunt('1SAT')}>
        <line x1="80" y1="250" x2="220" y2="250" stroke="#e2e8f0" stroke-width="3" />
        <line x1="220" y1="250" x2="480" y2="250" stroke="#e2e8f0" stroke-width="3" />
        <circle cx="150" cy="250" r="4.5" fill={trackOccupancy['1SAT'] ? '#ef4444' : '#1e293b'} stroke="#cbd5e1" stroke-width="1" />
        <text x="150" y="240" text-anchor="middle" fill={trackOccupancy['1SAT'] ? '#ef4444' : '#94a3b8'} font-size="9" font-family="monospace">1SAT</text>
      </g>
      <!-- Signal 2SA Dwarf on MT1 -->
      <circle cx="220" cy="268" r="4" fill={signalAspects['2SA'] !== 'Stop' ? '#22c55e' : '#ef4444'} stroke="#ffffff" stroke-width="1" />
      <text x="220" y="286" text-anchor="middle" fill="#e2e8f0" font-size="9" font-weight="700">2SA</text>

      <!-- Switch Point Indication Lights on Model Board (Opal/White) -->
      <!-- Switch 1 points indicator -->
      <circle cx="360" cy="160" r="5" fill={switchFieldStatus['1'] === 'Normal' ? '#ffffff' : '#334155'} stroke="#000000" stroke-width="1" />
      <!-- Switch 3 points indicator -->
      <circle cx="580" cy="160" r="5" fill={switchFieldStatus['3'] === 'Normal' ? '#ffffff' : '#334155'} stroke="#000000" stroke-width="1" />
      <!-- Derail 5 indicator -->
      <circle cx="460" cy="70" r="5" fill={switchFieldStatus['5'] === 'Normal' ? '#ef4444' : '#ffffff'} stroke="#000000" stroke-width="1" />
    </svg>
  </div>

  <!-- LOWER SECTION: The US&S Vertical Station Lever Deck -->
  <div class="lever-deck-section">
    <!-- STATION COLUMN 1 (Switch 1 & Signal 4) -->
    <div class="station-plate">
      <div class="column-nameplate">STATION 1</div>

      <!-- US&S Switch 1 Unit -->
      <div class="uss-lever-group">
        <div class="jewel-cluster">
          <div class="jewel-socket" title="Normal Correspondence (1NWK)">
            <span class="uss-jewel jewel-opal" class:lit={switchFieldStatus['1'] === 'Normal'}>●</span>
            <span class="jewel-tag">N</span>
          </div>
          <span class="plate-number">1</span>
          <div class="jewel-socket" title="Reverse Correspondence (1RWK)">
            <span class="uss-jewel jewel-amber" class:lit={switchFieldStatus['1'] === 'Reverse'}>●</span>
            <span class="jewel-tag">R</span>
          </div>
        </div>

        <!-- US&S Teardrop/Paddle Switch Lever (0° Normal up, 60° Reverse down-right) -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="uss-switch-lever"
          class:reverse={switchDemands['1'] === 'Reverse'}
          onclick={() => toggleSwitchLever('1')}
          oncontextmenu={(e) => cycleDog('switch', '1', e)}
          title="Click to throw (60° detent) | Right-click to apply/remove dog"
        >
          <div class="paddle-stem">
            <div class="paddle-blade"></div>
          </div>
          <div class="lever-center-hub"></div>
          {#if switchDogs['1'] !== 'none'}
            <div class="blocking-dog dog-{switchDogs['1']}">DOG</div>
          {/if}
        </div>
        <div class="detent-notches"><span>N</span><span>R</span></div>
      </div>

      <!-- US&S Signal 4 Unit -->
      <div class="uss-lever-group">
        <div class="jewel-cluster three-jewel">
          <div class="jewel-socket" title="Left Permissive (4NA)">
            <span class="uss-jewel jewel-green" class:lit={signalAspects['4NA'] !== 'Stop'}>●</span>
            <span class="jewel-tag">L</span>
          </div>
          <div class="jewel-socket" title="Stop Indication">
            <span class="uss-jewel jewel-red" class:lit={signalAspects['4NA'] === 'Stop' && signalAspects['4SA'] === 'Stop'}>●</span>
            <span class="jewel-tag">STOP</span>
          </div>
          <div class="jewel-socket" title="Right Permissive (4SA)">
            <span class="uss-jewel jewel-green" class:lit={signalAspects['4SA'] !== 'Stop'}>●</span>
            <span class="jewel-tag">R</span>
          </div>
        </div>

        <!-- US&S 3-Position Signal Lever (-45° Left, 0° Stop, +45° Right) -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="uss-signal-lever pos-{signalDemands['4'].toLowerCase()}"
          onclick={() => cycleSignalLever('4')}
          oncontextmenu={(e) => cycleDog('signal', '4', e)}
          title="Click to cycle L / STOP / R | Right-click to dog"
        >
          <div class="paddle-stem">
            <div class="paddle-blade"></div>
          </div>
          <div class="lever-center-hub"></div>
          {#if signalDogs['4'] !== 'none'}
            <div class="blocking-dog dog-{signalDogs['4']}">DOG</div>
          {/if}
        </div>
        <div class="detent-notches"><span>L</span><span>STOP</span><span>R</span></div>
      </div>

      <!-- US&S Machined Metal Code Button -->
      <button class="uss-code-button" onclick={() => punchCodeButton(1)} title="Punch to transmit atomic snapshot">
        <div class="button-inner">CODE 1</div>
      </button>
    </div>

    <!-- STATION COLUMN 2 (Switch 3 & Signal 2) -->
    <div class="station-plate">
      <div class="column-nameplate">STATION 2</div>

      <!-- US&S Switch 3 Unit -->
      <div class="uss-lever-group">
        <div class="jewel-cluster">
          <div class="jewel-socket" title="Normal Correspondence (3NWK)">
            <span class="uss-jewel jewel-opal" class:lit={switchFieldStatus['3'] === 'Normal'}>●</span>
            <span class="jewel-tag">N</span>
          </div>
          <span class="plate-number">3</span>
          <div class="jewel-socket" title="Reverse Correspondence (3RWK)">
            <span class="uss-jewel jewel-amber" class:lit={switchFieldStatus['3'] === 'Reverse'}>●</span>
            <span class="jewel-tag">R</span>
          </div>
        </div>

        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="uss-switch-lever"
          class:reverse={switchDemands['3'] === 'Reverse'}
          onclick={() => toggleSwitchLever('3')}
          oncontextmenu={(e) => cycleDog('switch', '3', e)}
          title="Click to throw | Right-click to dog"
        >
          <div class="paddle-stem">
            <div class="paddle-blade"></div>
          </div>
          <div class="lever-center-hub"></div>
          {#if switchDogs['3'] !== 'none'}
            <div class="blocking-dog dog-{switchDogs['3']}">DOG</div>
          {/if}
        </div>
        <div class="detent-notches"><span>N</span><span>R</span></div>
      </div>

      <!-- US&S Signal 2 Unit -->
      <div class="uss-lever-group">
        <div class="jewel-cluster three-jewel">
          <div class="jewel-socket" title="Left Permissive (2NAB)">
            <span class="uss-jewel jewel-green" class:lit={signalAspects['2NAB'] !== 'Stop'}>●</span>
            <span class="jewel-tag">L</span>
          </div>
          <div class="jewel-socket" title="Stop Indication">
            <span class="uss-jewel jewel-red" class:lit={signalAspects['2NAB'] === 'Stop' && signalAspects['2SA'] === 'Stop'}>●</span>
            <span class="jewel-tag">STOP</span>
          </div>
          <div class="jewel-socket" title="Right Permissive (2SA)">
            <span class="uss-jewel jewel-green" class:lit={signalAspects['2SA'] !== 'Stop'}>●</span>
            <span class="jewel-tag">R</span>
          </div>
        </div>

        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="uss-signal-lever pos-{signalDemands['2'].toLowerCase()}"
          onclick={() => cycleSignalLever('2')}
          oncontextmenu={(e) => cycleDog('signal', '2', e)}
          title="Click to cycle L / STOP / R | Right-click to dog"
        >
          <div class="paddle-stem">
            <div class="paddle-blade"></div>
          </div>
          <div class="lever-center-hub"></div>
          {#if signalDogs['2'] !== 'none'}
            <div class="blocking-dog dog-{signalDogs['2']}">DOG</div>
          {/if}
        </div>
        <div class="detent-notches"><span>L</span><span>STOP</span><span>R</span></div>
      </div>

      <!-- US&S Machined Metal Code Button -->
      <button class="uss-code-button" onclick={() => punchCodeButton(2)} title="Punch to transmit atomic snapshot">
        <div class="button-inner">CODE 2</div>
      </button>
    </div>

    <!-- STATION COLUMN 3 (Derail 5) -->
    <div class="station-plate">
      <div class="column-nameplate">STATION 3</div>

      <!-- US&S Switch 5 (Derail) Unit -->
      <div class="uss-lever-group">
        <div class="jewel-cluster">
          <div class="jewel-socket" title="Derail On (5NWK)">
            <span class="uss-jewel jewel-red" class:lit={switchFieldStatus['5'] === 'Normal'}>●</span>
            <span class="jewel-tag">ON</span>
          </div>
          <span class="plate-number">5</span>
          <div class="jewel-socket" title="Derail Off (5RWK)">
            <span class="uss-jewel jewel-opal" class:lit={switchFieldStatus['5'] === 'Reverse'}>●</span>
            <span class="jewel-tag">OFF</span>
          </div>
        </div>

        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="uss-switch-lever"
          class:reverse={switchDemands['5'] === 'Reverse'}
          onclick={() => toggleSwitchLever('5')}
          oncontextmenu={(e) => cycleDog('switch', '5', e)}
          title="Normal = Derail Active | Reverse = Derail Clear"
        >
          <div class="paddle-stem">
            <div class="paddle-blade"></div>
          </div>
          <div class="lever-center-hub"></div>
          {#if switchDogs['5'] !== 'none'}
            <div class="blocking-dog dog-{switchDogs['5']}">DOG</div>
          {/if}
        </div>
        <div class="detent-notches"><span>ON</span><span>OFF</span></div>
      </div>

      <!-- Blank Signal Plate for Derail Column -->
      <div class="uss-lever-group blank-lever-group">
        <div class="blank-indicator">[ BLANK ]</div>
      </div>

      <!-- US&S Machined Metal Code Button -->
      <button class="uss-code-button" onclick={() => punchCodeButton(3)} title="Punch to transmit atomic snapshot">
        <div class="button-inner">CODE 3</div>
      </button>
    </div>
  </div>
</div>

<style>
  .ctc-desk-root {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #06090f;
    color: #cbd5e1;
    user-select: none;
    overflow: hidden;
  }

  .faceplate-banner {
    background: #0f172a;
    border-bottom: 2px solid #1e293b;
    padding: 10px 24px;
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

  /* MODEL BOARD UPPER TIER */
  .model-board-section {
    flex: 1;
    background: #090d16;
    border-bottom: 4px solid #000000;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    padding: 12px;
  }

  .model-board-svg {
    width: 100%;
    max-width: 960px;
    height: 280px;
  }

  .track-block {
    cursor: pointer;
  }

  .track-block:hover line {
    stroke: #38bdf8;
  }

  /* LOWER TIER: US&S Vertical Station Lever Deck */
  .lever-deck-section {
    height: 350px;
    background: #101622;
    border-top: 3px solid #1e293b;
    display: flex;
    justify-content: center;
    gap: 40px;
    padding: 16px 24px;
    box-shadow: inset 0 6px 12px rgba(0, 0, 0, 0.6);
  }

  .station-plate {
    width: 170px;
    background: #182234;
    border: 3px solid #334155;
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 12px 10px;
    box-shadow: 0 10px 25px rgba(0, 0, 0, 0.6), inset 0 1px 2px rgba(255, 255, 255, 0.1);
  }

  .column-nameplate {
    font-size: 11px;
    font-weight: 800;
    color: #e2e8f0;
    letter-spacing: 0.6px;
    background: #0f172a;
    padding: 3px 14px;
    border-radius: 4px;
    border: 1px solid #334155;
    margin-bottom: 12px;
  }

  .uss-lever-group {
    display: flex;
    flex-direction: column;
    align-items: center;
    margin-bottom: 14px;
    width: 100%;
  }

  .jewel-cluster {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 16px;
    margin-bottom: 6px;
  }

  .jewel-cluster.three-jewel {
    gap: 10px;
  }

  .jewel-socket {
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .jewel-tag {
    font-size: 8px;
    font-weight: 800;
    color: #64748b;
    margin-top: 1px;
  }

  .plate-number {
    font-size: 12px;
    font-weight: 900;
    color: #cbd5e1;
  }

  /* Authentic US&S Faceted Glass Jewel Lamps with Chrome Rim */
  .uss-jewel {
    font-size: 16px;
    opacity: 0.25;
    transition: all 0.15s ease;
    filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.8));
  }

  .uss-jewel.lit {
    opacity: 1;
  }

  .jewel-opal.lit {
    color: #ffffff;
    text-shadow: 0 0 10px #ffffff, 0 0 20px #e0f2fe;
  }

  .jewel-amber.lit {
    color: #f59e0b;
    text-shadow: 0 0 10px #f59e0b, 0 0 20px #d97706;
  }

  .jewel-red.lit {
    color: #ef4444;
    text-shadow: 0 0 10px #ef4444, 0 0 20px #b91c1c;
  }

  .jewel-green.lit {
    color: #22c55e;
    text-shadow: 0 0 10px #22c55e, 0 0 20px #15803d;
  }

  /* Large Authentic US&S Teardrop/Paddle Handle Levers */
  .uss-switch-lever,
  .uss-signal-lever {
    width: 64px;
    height: 64px;
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: transform 0.22s cubic-bezier(0.34, 1.56, 0.64, 1);
  }

  /* Switch Lever: 0° Normal up, 60° Reverse down-right */
  .uss-switch-lever {
    transform: rotate(0deg);
  }

  .uss-switch-lever.reverse {
    transform: rotate(60deg);
  }

  /* Signal Lever: -45° Left, 0° Stop, +45° Right */
  .uss-signal-lever.pos-left {
    transform: rotate(-45deg);
  }

  .uss-signal-lever.pos-stop {
    transform: rotate(0deg);
  }

  .uss-signal-lever.pos-right {
    transform: rotate(45deg);
  }

  .paddle-stem {
    position: absolute;
    bottom: 24px;
    width: 10px;
    height: 36px;
    background: linear-gradient(90deg, #334155, #64748b, #334155);
    border-radius: 4px 4px 2px 2px;
    box-shadow: 0 3px 6px rgba(0, 0, 0, 0.6);
  }

  .paddle-blade {
    position: absolute;
    top: -8px;
    left: -4px;
    width: 18px;
    height: 14px;
    background: #cbd5e1;
    border-radius: 4px;
    border: 1px solid #94a3b8;
    box-shadow: inset 0 1px 2px rgba(255, 255, 255, 0.6);
  }

  .lever-center-hub {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: radial-gradient(circle, #e2e8f0 30%, #475569 80%);
    border: 2px solid #0f172a;
    box-shadow: 0 3px 6px rgba(0, 0, 0, 0.7);
    z-index: 2;
  }

  .detent-notches {
    display: flex;
    justify-content: space-between;
    width: 60px;
    font-size: 9px;
    font-weight: 800;
    color: #94a3b8;
    margin-top: 4px;
  }

  /* Large Machined Metal US&S Code Button */
  .uss-code-button {
    margin-top: auto;
    width: 100px;
    height: 38px;
    background: linear-gradient(180deg, #cbd5e1, #64748b);
    border: 2px solid #f1f5f9;
    border-radius: 6px;
    padding: 2px;
    cursor: pointer;
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.5), inset 0 1px 1px rgba(255, 255, 255, 0.8);
    transition: all 0.1s ease;
  }

  .uss-code-button:hover {
    filter: brightness(1.1);
  }

  .uss-code-button:active {
    transform: translateY(2px);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.5);
  }

  .button-inner {
    width: 100%;
    height: 100%;
    background: linear-gradient(180deg, #e2e8f0, #94a3b8);
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    font-weight: 900;
    color: #0f172a;
    letter-spacing: 0.5px;
  }

  .blank-lever-group {
    height: 80px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .blank-indicator {
    font-size: 10px;
    font-weight: 700;
    color: #475569;
  }

  /* Mechanical Blocking Dogs (Red / Blue Collars) */
  .blocking-dog {
    position: absolute;
    top: -8px;
    right: -12px;
    font-size: 8px;
    font-weight: 900;
    padding: 2px 4px;
    border-radius: 3px;
    color: #ffffff;
    box-shadow: 0 2px 5px rgba(0, 0, 0, 0.7);
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
