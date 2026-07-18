<script lang="ts">
  import { tick } from 'svelte';
  import { getDialogState } from '$lib/state/dialogState.svelte';

  const dialog = getDialogState();
  let inputValue = $state('');
  let inputElement = $state<HTMLInputElement>();

  $effect(() => {
    const request = dialog.request;
    if (request?.kind === 'prompt') {
      inputValue = request.value;
      tick().then(() => inputElement?.focus());
    }
  });

  function confirm() {
    const request = dialog.request;
    if (!request) return;
    dialog.clear();
    if (request.kind === 'confirm') request.resolve(true);
    else request.resolve(inputValue);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') dialog.cancel();
  }

  function handleBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) dialog.cancel();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if dialog.request}
  <div class="backdrop" role="presentation" onclick={handleBackdropClick}>
    <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="app-dialog-title">
      <form
        onsubmit={(event) => {
          event.preventDefault();
          confirm();
        }}
      >
        <h3 id="app-dialog-title">{dialog.request.title}</h3>
        {#if dialog.request.message}<p>{dialog.request.message}</p>{/if}
        {#if dialog.request.kind === 'prompt'}
          <input
            bind:this={inputElement}
            bind:value={inputValue}
            placeholder={dialog.request.placeholder ?? ''}
            aria-label={dialog.request.title}
          />
        {/if}
        <div class="actions">
          <button type="button" onclick={dialog.cancel}>取消</button>
          <button class:danger={dialog.request.kind === 'confirm' && dialog.request.danger}>
            {dialog.request.confirmLabel}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 3000;
    display: grid;
    place-items: center;
    padding: 20px;
    background: rgb(5 5 15 / 72%);
    backdrop-filter: blur(2px);
  }

  .dialog {
    width: min(440px, 100%);
    padding: 22px;
    border: 1px solid #3a3a5a;
    border-radius: 10px;
    background: #1e1e3a;
    box-shadow: 0 18px 55px rgb(0 0 0 / 45%);
    color: #eee;
  }

  h3 {
    margin: 0;
    font-size: 18px;
  }

  p {
    margin: 12px 0 0;
    color: #aaa;
    line-height: 1.55;
    overflow-wrap: anywhere;
  }

  input {
    box-sizing: border-box;
    width: 100%;
    margin-top: 16px;
    padding: 9px 11px;
    border: 1px solid #4a4a6a;
    border-radius: 6px;
    outline: none;
    background: #16162d;
    color: #eee;
  }

  input:focus {
    border-color: #e94560;
    box-shadow: 0 0 0 2px rgb(233 69 96 / 18%);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 9px;
    margin-top: 20px;
  }

  button {
    padding: 8px 14px;
    border: 0;
    border-radius: 5px;
    background: #2a2a4a;
    color: #ddd;
    cursor: pointer;
  }

  button:last-child {
    background: #e94560;
    color: #fff;
  }

  button.danger {
    background: #c73750;
  }
</style>
