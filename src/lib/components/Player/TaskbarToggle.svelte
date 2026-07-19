<script lang="ts">
  import { getTaskbarState } from '$lib/state/taskbarState.svelte';
  import { warnNonCritical } from '$lib/logic/error-handler';

  const taskbar = getTaskbarState();

  async function toggle() {
    try {
      await taskbar.setEnabled(!taskbar.enabled);
    } catch (error) {
      warnNonCritical('切換工作列播放器', error);
    }
  }
</script>

<button
  class="taskbar-toggle"
  class:active={taskbar.enabled}
  onclick={toggle}
  disabled={!taskbar.supported || taskbar.busy}
  aria-label={taskbar.enabled ? '關閉工作列播放器' : '開啟工作列播放器'}
  title={taskbar.supported ? taskbar.message : '工作列播放器僅支援 Windows'}
>
  <svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor" aria-hidden="true">
    <path d="M3 5h18v14H3V5zm2 2v8h14V7H5zm0 10h14v-1H5v1zm8-7 4 2.5-4 2.5v-5z" />
  </svg>
</button>

<style>
  .taskbar-toggle {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: #888;
    cursor: pointer;
  }

  .taskbar-toggle:hover:not(:disabled) {
    color: #ccc;
  }

  .taskbar-toggle.active {
    color: #e94560;
  }

  .taskbar-toggle:disabled {
    cursor: default;
    opacity: 0.35;
  }
</style>
