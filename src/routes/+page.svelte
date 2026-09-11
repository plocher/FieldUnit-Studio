<script lang="ts">
  import { onMount } from 'svelte';
  import { studio } from '$lib/state.svelte';
  import Toolbar from '$lib/components/Toolbar.svelte';
  import Palette from '$lib/components/Palette.svelte';
  import SchematicCanvas from '$lib/components/SchematicCanvas.svelte';
  import Inspector from '$lib/components/Inspector.svelte';
  import MatrixPanel from '$lib/components/MatrixPanel.svelte';

  onMount(() => {
    // Automatically load the CP End-of-Siding demo project at boot
    studio.loadDemo();
  });

  function handleKeyDown(event: KeyboardEvent) {
    // Ignore keystrokes when typing inside input or select
    const target = event.target as HTMLElement;
    if (target.tagName === 'INPUT' || target.tagName === 'SELECT' || target.tagName === 'TEXTAREA') {
      return;
    }

    switch (event.key.toLowerCase()) {
      case 't':
        studio.toggleLayer('track');
        break;
      case 'e':
        studio.toggleLayer('electrical');
        break;
      case 's':
        studio.toggleLayer('signals');
        break;
      case 'r':
        studio.toggleLayer('speeds');
        break;
      case 'n':
        studio.toggleLayer('names');
        break;
      case '0':
        if (event.metaKey || event.ctrlKey) {
          event.preventDefault();
          studio.isLeftSidebarOpen = !studio.isLeftSidebarOpen;
        }
        break;
      case 'j':
        if (event.metaKey || event.ctrlKey) {
          event.preventDefault();
          studio.isBottomPanelOpen = !studio.isBottomPanelOpen;
        }
        break;
    }
  }
</script>

<svelte:window onkeydown={handleKeyDown} />

<div class="studio-root">
  <Toolbar />

  <div class="workspace-body">
    {#if studio.workspaceMode === 'schematic'}
      <div class="editor-layout">
        {#if studio.isLeftSidebarOpen}
          <Palette />
        {/if}

        <SchematicCanvas />

        {#if studio.isRightSidebarOpen}
          <Inspector />
        {/if}
      </div>

      <MatrixPanel />
    {:else if studio.workspaceMode === 'matrix'}
      <!-- Dedicated Matrix View -->
      <div class="full-view-panel">
        <MatrixPanel />
      </div>
    {:else if studio.workspaceMode === 'ctc'}
      <!-- Virtual cTc Console Preview (Phase 3 Stub) -->
      <div class="ctc-preview-container">
        <div class="ctc-faceplate">
          <div class="ctc-header">US&S STYLE 504 cTc DISPATCHER DESK [PREVIEW]</div>
          <div class="ctc-status-line">Connected: Simulated In-Memory Plant | Stepper Relays: Idle</div>
          <div class="ctc-desk-mock">
            <div class="ctc-model-board">
              <div class="board-track-line">
                <span class="lamp lamp-red" title="1T1 Island Shunted">●</span>
                <span class="track-segment">════════════════════</span>
                <span class="lamp lamp-green" title="Signal 2R Cleared">●</span>
                <span class="track-segment">═══════════════</span>
              </div>
            </div>
            <div class="ctc-lever-deck">
              <div class="lever-column">
                <div class="lamp lamp-white">●</div>
                <div class="lever-switch">▲ N</div>
                <div class="lever-label">SW 1</div>
                <div class="code-btn">CODE</div>
              </div>
              <div class="lever-column">
                <div class="lamp lamp-green">●</div>
                <div class="lever-signal">◀ L | STOP | R ▶</div>
                <div class="lever-label">SIG 2</div>
                <div class="code-btn">CODE</div>
              </div>
            </div>
          </div>
        </div>
      </div>
    {:else if studio.workspaceMode === 'code'}
      <!-- Code & Documentation Export Preview (Phase 4 Stub) -->
      <div class="code-preview-container">
        <div class="code-box">
          <div class="code-header">Generated C++ configurePlant() Code</div>
          <pre class="code-content"><code>{`void configurePlant(ControlPoint& cp, CodeLineCodec& codec) {
    // 1. Declare Appliances
    auto sw1 = cp.addSwitch("SW1", PIN_SW1_MOTOR, PIN_SW1_NORM, PIN_SW1_REV);
    auto tc1T1 = cp.addTrackCircuit("1T1", PIN_TC_1T1, /*dropoutDelayMs=*/2000);
    auto sig2R = cp.addSignalMast("2R", MastType::TWO_HEAD, Direction::RIGHT);

    // 2. Interlocking Control Table
    cp.route("Main-to-Main")
      .governedBy(sig2R, Direction::RIGHT)
      .aligns({ {sw1, SwitchPosition::NORMAL} })
      .clears({ tc1T1 })
      .aspectCeiling(Indication::CLEAR);

    cp.route("Main-to-Siding")
      .governedBy(sig2R, Direction::RIGHT)
      .aligns({ {sw1, SwitchPosition::REVERSE} })
      .clears({ tc1T1 })
      .aspectCeiling(Indication::DIVERGING_CLEAR);
}`}</code></pre>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
    background-color: #090d16;
    color: #f8fafc;
    overflow: hidden;
  }

  .studio-root {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
  }

  .workspace-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .editor-layout {
    flex: 1;
    display: flex;
    overflow: hidden;
    position: relative;
  }

  .full-view-panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    background: #0f172a;
    padding: 20px;
  }

  .ctc-preview-container,
  .code-preview-container {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #090d16;
    padding: 40px;
  }

  .ctc-faceplate {
    background: #181c24;
    border: 3px solid #334155;
    border-radius: 8px;
    padding: 24px;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.6);
    text-align: center;
  }

  .ctc-header {
    font-size: 14px;
    font-weight: 800;
    letter-spacing: 1px;
    color: #f1f5f9;
  }

  .ctc-status-line {
    font-size: 11px;
    color: #64748b;
    margin: 6px 0 20px;
  }

  .ctc-desk-mock {
    display: flex;
    flex-direction: column;
    gap: 30px;
    background: #0b0f19;
    padding: 24px;
    border-radius: 6px;
    border: 1px solid #1e293b;
  }

  .board-track-line {
    font-family: monospace;
    font-size: 16px;
    color: #cbd5e1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
  }

  .lamp {
    font-size: 14px;
  }

  .lamp-red {
    color: #ef4444;
    text-shadow: 0 0 8px #ef4444;
  }

  .lamp-green {
    color: #22c55e;
    text-shadow: 0 0 8px #22c55e;
  }

  .lamp-white {
    color: #ffffff;
    text-shadow: 0 0 8px #ffffff;
  }

  .ctc-lever-deck {
    display: flex;
    justify-content: center;
    gap: 40px;
  }

  .lever-column {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    background: #131926;
    padding: 12px 18px;
    border-radius: 6px;
    border: 1px solid #1e293b;
  }

  .lever-switch,
  .lever-signal {
    background: #1e293b;
    padding: 6px 12px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 700;
    color: #f8fafc;
    border: 1px solid #475569;
  }

  .lever-label {
    font-size: 10px;
    font-weight: 700;
    color: #94a3b8;
  }

  .code-btn {
    background: linear-gradient(180deg, #94a3b8, #64748b);
    color: #0f172a;
    font-weight: 800;
    font-size: 9px;
    padding: 4px 8px;
    border-radius: 50%;
    border: 2px solid #cbd5e1;
    cursor: pointer;
  }

  .code-box {
    width: 650px;
    background: #0f172a;
    border: 1px solid #1e293b;
    border-radius: 8px;
    overflow: hidden;
  }

  .code-header {
    background: #1e293b;
    padding: 10px 16px;
    font-size: 12px;
    font-weight: 700;
    color: #38bdf8;
  }

  .code-content {
    margin: 0;
    padding: 16px;
    font-family: 'Fira Code', monospace;
    font-size: 12px;
    line-height: 1.5;
    color: #e2e8f0;
    overflow-x: auto;
  }
</style>
