import * as taskbarApi from '$lib/api/taskbar';
import type { TaskbarPreferenceMode, TaskbarStatus } from '$lib/types';

let status = $state<TaskbarStatus>({
  supported: false,
  enabled: false,
  running: false,
  visible: false,
  mode: null,
  message: '正在讀取 Windows 整合狀態…',
});
let initialized = $state(false);
let busy = $state(false);
let preferenceMode = $state<TaskbarPreferenceMode>('auto');
let offsetX = $state(0);
let showTitleMarquee = $state(true);
let showProgress = $state(true);
let hideInMiniPlayer = $state(true);
let initialization: Promise<void> | null = null;

async function initialize(): Promise<void> {
  if (initialization) return initialization;
  initialization = (async () => {
    const [settings, currentStatus] = await Promise.all([
      taskbarApi.getTaskbarSettings(),
      taskbarApi.getTaskbarStatus(),
    ]);
    status = { ...currentStatus, enabled: settings.enabled && currentStatus.supported };
    preferenceMode = settings.mode;
    offsetX = settings.offset_x;
    showTitleMarquee = settings.show_title_marquee;
    showProgress = settings.show_progress;
    hideInMiniPlayer = settings.hide_in_mini_player;
    initialized = true;
  })().finally(() => {
    initialization = null;
  });
  return initialization;
}

async function setMode(mode: TaskbarPreferenceMode): Promise<void> {
  busy = true;
  try {
    status = await taskbarApi.setTaskbarPlayerMode(mode);
    preferenceMode = mode;
  } finally {
    busy = false;
  }
}

async function setOffset(nextOffsetX: number): Promise<void> {
  busy = true;
  try {
    offsetX = Number.isFinite(nextOffsetX)
      ? Math.max(-1600, Math.min(0, Math.round(nextOffsetX)))
      : 0;
    status = await taskbarApi.setTaskbarPlayerOffset(offsetX);
  } finally {
    busy = false;
  }
}

async function setEnabled(enabled: boolean): Promise<void> {
  busy = true;
  try {
    status = await taskbarApi.setTaskbarPlayerEnabled(enabled);
    initialized = true;
  } finally {
    busy = false;
  }
}

async function setDisplayOptions(
  nextShowTitleMarquee: boolean,
  nextShowProgress: boolean,
): Promise<void> {
  busy = true;
  try {
    const settings = await taskbarApi.setTaskbarPlayerDisplayOptions(
      nextShowTitleMarquee,
      nextShowProgress,
    );
    showTitleMarquee = settings.show_title_marquee;
    showProgress = settings.show_progress;
  } finally {
    busy = false;
  }
}

async function setMiniModeBehavior(nextHideInMiniPlayer: boolean): Promise<void> {
  busy = true;
  try {
    const settings = await taskbarApi.setTaskbarPlayerMiniModeBehavior(nextHideInMiniPlayer);
    hideInMiniPlayer = settings.hide_in_mini_player;
  } finally {
    busy = false;
  }
}

export function applyTaskbarStatus(nextStatus: TaskbarStatus): void {
  status = nextStatus;
  initialized = true;
}

export function getTaskbarState() {
  return {
    get status() {
      return status;
    },
    get supported() {
      return status.supported;
    },
    get enabled() {
      return status.enabled;
    },
    get running() {
      return status.running;
    },
    get visible() {
      return status.visible;
    },
    get mode() {
      return status.mode;
    },
    get message() {
      return status.message;
    },
    get preferenceMode() {
      return preferenceMode;
    },
    get offsetX() {
      return offsetX;
    },
    get showTitleMarquee() {
      return showTitleMarquee;
    },
    get showProgress() {
      return showProgress;
    },
    get hideInMiniPlayer() {
      return hideInMiniPlayer;
    },
    get initialized() {
      return initialized;
    },
    get busy() {
      return busy;
    },
    initialize,
    setEnabled,
    setMode,
    setOffset,
    setDisplayOptions,
    setMiniModeBehavior,
  };
}
