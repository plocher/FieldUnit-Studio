<script lang="ts">
  import { onMount } from 'svelte';
  import { studio } from '$lib/state.svelte';
  import Toolbar from '$lib/components/Toolbar.svelte';
  import Palette from '$lib/components/Palette.svelte';
  import SchematicCanvas from '$lib/components/SchematicCanvas.svelte';
  import Inspector from '$lib/components/Inspector.svelte';
  import MatrixPanel from '$lib/components/MatrixPanel.svelte';
  import CtcConsole from '$lib/components/CtcConsole.svelte';

  onMount(() => {
    // Automatically load the CP End-of-Siding demo project at boot
    studio.loadDemo();
  });

  function handleKeyDown(event: KeyboardEvent) {
    const target = event.target as HTMLElement;
    if (target.tagName === 'INPUT' || target.tagName === 'SELECT' || target.tagName === 'TEXTAREA') {
      return;
    }

    // Undo / Redo
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'z') {
      event.preventDefault();
      if (event.shiftKey) {
        studio.redo();
      } else {
        studio.undo();
      }
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
        // KiCad-style 'R' key: Rotate facing direction East <-> West
        studio.rotateSelectedSwitch();
        break;
      case 'f':
        // KiCad-style 'F' key: Flip diverge side Up <-> Down
        studio.flipSelectedSwitch();
        break;
      case 'p':
        studio.toggleLayer('speeds');
        break;
      case 'n':
        studio.toggleLayer('names');
        break;
      case 'u':
        // KiCad-style 'U' key: extend selection along same corridor (Shift+U crosses corridor levels)
        studio.extendSelection(event.shiftKey);
        break;
      case 'l':
        // Auto-arrange / tidy layout
        studio.autoArrange();
        break;
      case 'delete':
      case 'backspace':
      case 'x':
        studio.deleteSelected();
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
      <!-- Virtual cTc Machine Console (Problem B & C) -->
      <CtcConsole />
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

  .code-preview-container {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #090d16;
    padding: 40px;
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
