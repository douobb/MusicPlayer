const CONFIRM_DELETIONS_KEY = 'musicplayer.confirm-deletions';

function loadConfirmDeletions(): boolean {
  if (typeof localStorage === 'undefined') return true;
  return localStorage.getItem(CONFIRM_DELETIONS_KEY) !== 'false';
}

let confirmDeletions = $state(loadConfirmDeletions());

export function getPreferencesState() {
  return {
    get confirmDeletions() {
      return confirmDeletions;
    },
    set confirmDeletions(value: boolean) {
      confirmDeletions = value;
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem(CONFIRM_DELETIONS_KEY, String(value));
      }
    },
  };
}
