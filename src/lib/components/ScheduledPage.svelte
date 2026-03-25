<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";

  interface QueueItem {
    id: string;
    audio_path: string;
    status: string;
    created_at: string;
    completed_at: string | null;
    transcript_path: string | null;
    error_message: string | null;
    duration: number | null;
  }

  let items = $state<QueueItem[]>([]);
  let isProcessing = $state(false);

  async function loadQueue() {
    try { items = await invoke("get_queue"); } catch (e) { console.error(e); }
  }

  async function transcribeAll() {
    isProcessing = true;
    try { await invoke("process_pending_queue"); } catch (e) { console.error(e); }
    isProcessing = false;
  }

  async function retryItem(id: string) {
    try { await invoke("retry_queue_item", { id }); } catch (e) { console.error(e); }
  }

  async function removeItem(id: string) {
    try { await invoke("remove_queue_item", { id }); } catch (e) { console.error(e); }
  }

  function formatDate(dateStr: string): string {
    try { return new Date(dateStr).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }); } catch { return dateStr; }
  }

  function fileName(path: string): string {
    return path.split(/[\\/]/).pop() || path;
  }

  onMount(() => {
    loadQueue();
    const unlisten = listen("queue-updated", () => loadQueue());
    return () => { unlisten.then((f) => f()); };
  });
</script>

<div class="flex-1 flex flex-col h-full bg-content overflow-auto">
  <div class="flex items-center justify-between px-5 pt-4 pb-3">
    <h1 class="text-lg font-semibold text-text-primary">Scheduled</h1>
    {#if items.some((i) => i.status === "Pending")}
      <button
        onclick={transcribeAll}
        disabled={isProcessing}
        class="px-3 py-1.5 text-[12px] bg-accent hover:bg-accent-hover text-white rounded-lg transition-colors disabled:opacity-50"
      >
        {isProcessing ? "Processing..." : "Transcribe All"}
      </button>
    {/if}
  </div>

  <div class="flex-1 overflow-auto px-4 pb-4">
    {#if items.length === 0}
      <div class="flex flex-col items-center justify-center h-full gap-2 text-text-muted">
        <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/>
        </svg>
        <p class="text-[13px]">No scheduled transcriptions</p>
        <p class="text-[11px]">Recordings will appear here</p>
      </div>
    {:else}
      <div class="space-y-2">
        {#each items as item (item.id)}
          <div class="bg-surface rounded-xl border border-border p-3
            {item.status === 'Failed' ? 'border-l-2 border-l-recording' : ''}
            {item.status === 'Processing' ? 'border-l-2 border-l-accent' : ''}">
            <div class="flex items-start justify-between gap-2">
              <div class="flex-1 min-w-0">
                <p class="text-[13px] text-text-primary truncate">{fileName(item.audio_path)}</p>
                <div class="flex items-center gap-2 mt-1">
                  <span class="text-[11px] px-1.5 py-0.5 rounded-md
                    {item.status === 'Completed' ? 'bg-green-500/15 text-green-400' :
                     item.status === 'Failed' ? 'bg-recording/15 text-recording' :
                     item.status === 'Processing' ? 'bg-accent/15 text-accent' :
                     'bg-text-muted/15 text-text-muted'}">
                    {item.status}
                  </span>
                  {#if item.duration}
                    <span class="text-[11px] text-text-muted">{item.duration.toFixed(0)}s</span>
                  {/if}
                  <span class="text-[11px] text-text-muted">{formatDate(item.created_at)}</span>
                </div>
                {#if item.error_message}
                  <p class="text-[11px] text-recording mt-1 truncate">{item.error_message}</p>
                {/if}
              </div>
              <div class="flex gap-1 flex-shrink-0">
                {#if item.status === "Failed"}
                  <button onclick={() => retryItem(item.id)}
                    class="px-2 py-1 text-[11px] text-accent hover:text-accent-hover transition-colors">
                    Retry
                  </button>
                {/if}
                {#if item.status !== "Processing"}
                  <button onclick={() => removeItem(item.id)}
                    class="px-1.5 py-1 text-[11px] text-text-muted hover:text-recording transition-colors">
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 6 6 18M6 6l12 12"/></svg>
                  </button>
                {/if}
              </div>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
