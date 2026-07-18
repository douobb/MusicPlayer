import { getPreferencesState } from './preferencesState.svelte';

export type DialogRequest =
  | {
      kind: 'confirm';
      title: string;
      message: string;
      confirmLabel: string;
      danger: boolean;
      resolve: (confirmed: boolean) => void;
    }
  | {
      kind: 'prompt';
      title: string;
      message?: string;
      value: string;
      placeholder?: string;
      confirmLabel: string;
      resolve: (value: string | null) => void;
    };

let request = $state<DialogRequest | null>(null);

function cancelCurrent(): void {
  if (!request) return;
  if (request.kind === 'confirm') request.resolve(false);
  else request.resolve(null);
  request = null;
}

export function getDialogState() {
  return {
    get request() {
      return request;
    },
    clear() {
      request = null;
    },
    cancel: cancelCurrent,
  };
}

export function askConfirmation(options: {
  title: string;
  message: string;
  confirmLabel?: string;
  danger?: boolean;
  destructive?: boolean;
}): Promise<boolean> {
  if (options.destructive && !getPreferencesState().confirmDeletions) return Promise.resolve(true);
  cancelCurrent();
  return new Promise((resolve) => {
    request = {
      kind: 'confirm',
      title: options.title,
      message: options.message,
      confirmLabel: options.confirmLabel ?? '確定',
      danger: options.danger ?? false,
      resolve,
    };
  });
}

export function askText(options: {
  title: string;
  message?: string;
  value?: string;
  placeholder?: string;
  confirmLabel?: string;
}): Promise<string | null> {
  cancelCurrent();
  return new Promise((resolve) => {
    request = {
      kind: 'prompt',
      title: options.title,
      message: options.message,
      value: options.value ?? '',
      placeholder: options.placeholder,
      confirmLabel: options.confirmLabel ?? '確定',
      resolve,
    };
  });
}
