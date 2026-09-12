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

  // Toggle switch lever (60° detent throw)
  function toggleSwitchLever(swId: string) {
    if (switchDogs[swId] !== 'none') return; // Blocked by mechanical dog
    switchDemands[swId] = switchDemands[swId] === 'Normal' ? 'Reverse' : 'Normal';
  }

  // Cycle signal lever (45° detents: Left -> Stop -> Right -> Stop)
  function cycleSignalLever(sigId: string) {
    if (signalDogs[sigId] !== 'none') return;
    const current = signalDemands[sigId];
    if (current === 'Stop') signalDemands[sigId] = 'Left';
    else if (current === 'Left') signalDemands[sigId] = 'Right';
    else signalDemands[sigId] = 'Stop';
  }

  // Clamp / remove mechanical blocking dog on right click
  function cycleDog(type: 'switch' | 'signal', id: string, event: MouseEvent) {
    event.preventDefault();
    if (type === 'switch') {
      const curr = switchDogs[id];
      switchDogs[id] = curr === 'none' ? 'red' : curr === 'red' ? 'blue' : 'none';
    } else {
      const curr = signalDogs[id];
      signalDogs[id] = curr === 'none' ? 'red' : curr === 'red' ? 'blue' : 'none';
    }
  }

  // Toggle track occupancy (interactive shunt testing)
  function toggleShunt(circuitId: string) {
    trackOccupancy[circuitId] = !trackOccupancy[circuitId];

    // Vital safety: if train shunts an island while signal is clear, instant knockdown to Stop!
    if (trackOccupancy['3T1'] || trackOccupancy['1T1']) {
      signalAspects['2NAB'] = 'Stop';
      signalAspects['2SA'] = 'Stop';
    }
    if (trackOccupancy['5T1'] || trackOccupancy['1T1']) {
      signalAspects['4NA'] = 'Stop';
      signalAspects['4SA'] = 'Stop';
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
      switchFieldStatus[swId] = 'Moving';
      // Simulate Tortoise motor travel time (2.0 seconds)
      setTimeout(() => {
        switchFieldStatus[swId] = demandedPos;
        evaluatePlantRoutes();
      }, 2000);
    }

    // 2. Process Signal Command with Approach Time Locking
    if (sigId) {
      const demandedSig = signalDemands[sigId];
      // If signal was previously permissive and dispatcher forces it to Stop: engage time lock!
      if (demandedSig === 'Stop' && (signalAspects['2NAB'] !== 'Stop' || signalAspects['2SA'] !== 'Stop')) {
        timeLockSeconds[sigId] = 15; // 15s time lock countdown for simulation
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

  <!-- LOWER SECTION: The Vertical cTc Station Lever Deck -->
  <div class="lever-deck-section">
    <!-- STATION COLUMN 1 (Switch 1 & Signal 4) -->
    <div class="station-plate">
      <div class="plate-label">STATION 1</div>
      <!-- Switch 1 Section -->
      <div class="lever-slot switch-slot">
        <div class="lamp-row">
          <span class="lamp lamp-white" class:lit={switchFieldStatus['1'] === 'Normal'}>●</span>
          <span class="lamp-name">1</span>
          <span class="lamp lamp-amber" class:lit={switchFieldStatus['1'] === 'Reverse'}>●</span>
        </div>
        <!-- 60° Detent Switch Lever -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="switch-lever"
          class:reverse={switchDemands['1'] === 'Reverse'}
          onclick={() => toggleSwitchLever('1')}
          oncontextmenu={(e) => cycleDog('switch', '1', e)}
          title="Click to throw (60° detent) | Right-click to apply/remove dog"
        >
          <div class="lever-handle">
            <div class="lever-pointer">▲</div>
          </div>
          {#if switchDogs['1'] !== 'none'}
            <div class="blocking-dog dog-{switchDogs['1']}">DOG</div>
          {/if}
        </div>
        <div class="detent-labels"><span>N</span><span>R</span></div>
        <div class="appliance-tag">SW 1</div>
      </div>

      <!-- Signal 4 Section -->
      <div class="lever-slot signal-slot">
        <div class="lamp-row">
          <span class="lamp lamp-green" class:lit={signalAspects['4NA'] !== 'Stop'}>●</span>
          <span class="lamp lamp-red" class:lit={signalAspects['4NA'] === 'Stop' && signalAspects['4SA'] === 'Stop'}>●</span>
          <span class="lamp lamp-green" class:lit={signalAspects['4SA'] !== 'Stop'}>●</span>
        </div>
        <!-- 45° Detent Signal Lever -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="signal-lever pos-{signalDemands['4'].toLowerCase()}"
          onclick={() => cycleSignalLever('4')}
          oncontextmenu={(e) => cycleDog('signal', '4', e)}
          title="Click to cycle L / STOP / R | Right-click to dog"
        >
          <div class="lever-handle">
            <div class="lever-pointer">●</div>
          </div>
          {#if signalDogs['4'] !== 'none'}
            <div class="blocking-dog dog-{signalDogs['4']}">DOG</div>
          {/if}
        </div>
        <div class="detent-labels"><span>L</span><span>STOP</span><span>R</span></div>
        <div class="appliance-tag">SIG 4</div>
      </div>

      <!-- Code Button -->
      <button class="code-button" onclick={() => punchCodeButton(1)} title="Punch to transmit Interface A snapshot">
        CODE 1
      </button>
    </div>

    <!-- STATION COLUMN 2 (Switch 3 & Signal 2) -->
    <div class="station-plate">
      <div class="plate-label">STATION 2</div>
      <!-- Switch 3 Section -->
      <div class="lever-slot switch-slot">
        <div class="lamp-row">
          <span class="lamp lamp-white" class:lit={switchFieldStatus['3'] === 'Normal'}>●</span>
          <span class="lamp-name">3</span>
          <span class="lamp lamp-amber" class:lit={switchFieldStatus['3'] === 'Reverse'}>●</span>
        </div>
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="switch-lever"
          class:reverse={switchDemands['3'] === 'Reverse'}
          onclick={() => toggleSwitchLever('3')}
          oncontextmenu={(e) => cycleDog('switch', '3', e)}
          title="Click to throw | Right-click to dog"
        >
          <div class="lever-handle">
            <div class="lever-pointer">▲</div>
          </div>
          {#if switchDogs['3'] !== 'none'}
            <div class="blocking-dog dog-{switchDogs['3']}">DOG</div>
          {/if}
        </div>
        <div class="detent-labels"><span>N</span><span>R</span></div>
        <div class="appliance-tag">SW 3</div>
      </div>

      <!-- Signal 2 Section -->
      <div class="lever-slot signal-slot">
        <div class="lamp-row">
          <span class="lamp lamp-green" class:lit={signalAspects['2NAB'] !== 'Stop'}>●</span>
          <span class="lamp lamp-red" class:lit={signalAspects['2NAB'] === 'Stop' && signalAspects['2SA'] === 'Stop'}>●</span>
          <span class="lamp lamp-green" class:lit={signalAspects['2SA'] !== 'Stop'}>●</span>
        </div>
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="signal-lever pos-{signalDemands['2'].toLowerCase()}"
          onclick={() => cycleSignalLever('2')}
          oncontextmenu={(e) => cycleDog('signal', '2', e)}
          title="Click to cycle L / STOP / R | Right-click to dog"
        >
          <div class="lever-handle">
            <div class="lever-pointer">●</div>
          </div>
          {#if signalDogs['2'] !== 'none'}
            <div class="blocking-dog dog-{signalDogs['2']}">DOG</div>
          {/if}
        </div>
        <div class="detent-labels"><span>L</span><span>STOP</span><span>R</span></div>
        <div class="appliance-tag">SIG 2</div>
      </div>

      <!-- Code Button -->
      <button class="code-button" onclick={() => punchCodeButton(2)} title="Punch to transmit Interface A snapshot">
        CODE 2
      </button>
    </div>

    <!-- STATION COLUMN 3 (Derail 5) -->
    <div class="station-plate">
      <div class="plate-label">STATION 3</div>
      <!-- Switch 5 (Derail) Section -->
      <div class="lever-slot switch-slot">
        <div class="lamp-row">
          <span class="lamp lamp-red" class:lit={switchFieldStatus['5'] === 'Normal'} title="Derail Active">●</span>
          <span class="lamp-name">5</span>
          <span class="lamp lamp-white" class:lit={switchFieldStatus['5'] === 'Reverse'} title="Derail Off">●</span>
        </div>
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="switch-lever"
          class:reverse={switchDemands['5'] === 'Reverse'}
          onclick={() => toggleSwitchLever('5')}
          oncontextmenu={(e) => cycleDog('switch', '5', e)}
          title="Normal = Derailing (Up) | Reverse = Aligned (Down)"
        >
          <div class="lever-handle">
            <div class="lever-pointer">▲</div>
          </div>
          {#if switchDogs['5'] !== 'none'}
            <div class="blocking-dog dog-{switchDogs['5']}">DOG</div>
          {/if}
        </div>
        <div class="detent-labels"><span>ON</span><span>OFF</span></div>
        <div class="appliance-tag">DERAIL 5</div>
      </div>

      <!-- Blank Signal Plate -->
      <div class="lever-slot blank-slot">
        <div class="blank-plate">[ BLANK ]</div>
      </div>

      <!-- Code Button -->
      <button class="code-button" onclick={() => punchCodeButton(3)} title="Punch to transmit Interface A snapshot">
        CODE 3
      </button>
    </div>
  </div>
</div>

<style>
  .ctc-desk-root {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #090d16;
    color: #cbd5e1;
    user-select: none;
    overflow: hidden;
  }

  .faceplate-banner {
    background: #0f172a;
    border-bottom: 2px solid #1e293b;
    padding: 10px 20px;
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

  .model-board-section {
    flex: 1;
    background: #0b0f19;
    border-bottom: 3px solid #1e293b;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    padding: 10px;
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

  /* LOWER SECTION: Station Lever Deck */
  .lever-deck-section {
    height: 280px;
    background: #111827;
    border-top: 2px solid #000000;
    display: flex;
    justify-content: center;
    gap: 30px;
    padding: 16px 20px;
  }

  .station-plate {
    width: 130px;
    background: #182234;
    border: 2px solid #334155;
    border-radius: 6px;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 10px 8px;
    box-shadow: inset 0 2px 4px rgba(255, 255, 255, 0.05), 0 8px 16px rgba(0, 0, 0, 0.5);
  }

  .plate-label {
    font-size: 10px;
    font-weight: 800;
    color: #94a3b8;
    letter-spacing: 0.5px;
    margin-bottom: 6px;
  }

  .lever-slot {
    display: flex;
    flex-direction: column;
    align-items: center;
    margin-bottom: 12px;
  }

  .lamp-row {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 4px;
  }

  .lamp {
    font-size: 13px;
    opacity: 0.25;
    transition: opacity 0.15s ease;
  }

  .lamp.lit {
    opacity: 1;
  }

  .lamp-white.lit {
    color: #ffffff;
    text-shadow: 0 0 8px #ffffff;
  }

  .lamp-amber.lit {
    color: #f59e0b;
    text-shadow: 0 0 8px #f59e0b;
  }

  .lamp-red.lit {
    color: #ef4444;
    text-shadow: 0 0 8px #ef4444;
  }

  .lamp-green.lit {
    color: #22c55e;
    text-shadow: 0 0 8px #22c55e;
  }

  .lamp-name {
    font-size: 10px;
    font-weight: 700;
    color: #94a3b8;
  }

  /* 60° Detent Switch Lever */
  .switch-lever {
    width: 38px;
    height: 38px;
    border-radius: 50%;
    background: #0f172a;
    border: 2px solid #475569;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    position: relative;
    transition: transform 0.2s cubic-bezier(0.34, 1.56, 0.64, 1);
    transform: rotate(0deg);
  }

  .switch-lever.reverse {
    transform: rotate(60deg);
  }

  .lever-handle {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: #cbd5e1;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.4);
  }

  .lever-pointer {
    font-size: 8px;
    color: #0f172a;
    font-weight: 900;
  }

  /* 45° Detent Signal Lever */
  .signal-lever {
    width: 38px;
    height: 38px;
    border-radius: 50%;
    background: #0f172a;
    border: 2px solid #475569;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    position: relative;
    transition: transform 0.2s cubic-bezier(0.34, 1.56, 0.64, 1);
  }

  .signal-lever.pos-left {
    transform: rotate(-45deg);
  }

  .signal-lever.pos-stop {
    transform: rotate(0deg);
  }

  .signal-lever.pos-right {
    transform: rotate(45deg);
  }

  .detent-labels {
    display: flex;
    justify-content: space-between;
    width: 50px;
    font-size: 8px;
    font-weight: 700;
    color: #64748b;
    margin-top: 2px;
  }

  .appliance-tag {
    font-size: 9px;
    font-weight: 800;
    color: #38bdf8;
    margin-top: 2px;
  }

  .code-button {
    margin-top: auto;
    background: linear-gradient(180deg, #94a3b8, #64748b);
    border: 2px solid #e2e8f0;
    color: #0f172a;
    font-size: 10px;
    font-weight: 900;
    padding: 6px 14px;
    border-radius: 4px;
    cursor: pointer;
    box-shadow: 0 3px 6px rgba(0, 0, 0, 0.5);
  }

  .code-button:hover {
    background: #cbd5e1;
  }

  .code-button:active {
    transform: translateY(2px);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.5);
  }

  .blank-slot {
    height: 60px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .blank-plate {
    font-size: 9px;
    color: #475569;
    font-weight: 600;
  }

  /* Mechanical Blocking Dogs */
  .blocking-dog {
    position: absolute;
    top: -6px;
    right: -10px;
    font-size: 7px;
    font-weight: 900;
    padding: 1px 3px;
    border-radius: 2px;
    color: #ffffff;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.5);
  }

  .dog-red {
    background: #dc2626;
  }

  .dog-blue {
    background: #2563eb;
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
