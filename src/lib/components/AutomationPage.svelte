<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Card from "./ui/Card.svelte";
  import type { HookConfig } from "$lib/stores/settings.svelte";

  let summaryEnabled = $state(false);
  let summaryDestination = $state("same");
  let summaryProvider = $state("claude_cli");
  let summaryPrompt = $state("Summarize this meeting transcript concisely. Include key decisions, action items, and main topics discussed.");
  let claudeAvailable = $state(false);
  let providerKeySet = $state(false);
  let hooks = $state<HookConfig[]>([]);
  let newHookName = $state("");
  let newHookPath = $state("");

  async function loadSettings() {
    try {
      const s: any = await invoke("get_settings");
      summaryEnabled = s.ai_summary_enabled;
      summaryDestination = s.ai_summary_destination;
      summaryProvider = s.ai_summary_provider || "claude_cli";
      summaryPrompt = s.ai_summary_prompt || "Summarize this meeting transcript concisely. Include key decisions, action items, and main topics discussed.";
      hooks = s.hooks || [];
      // Check if the selected API provider has a key configured
      if (summaryProvider !== "claude_cli") {
        const key = s.api_keys?.[summaryProvider] || "";
        providerKeySet = key.length > 0;
      }
    } catch (e) { console.error(e); }
    try { claudeAvailable = await invoke("check_claude_cli"); } catch { claudeAvailable = false; }
  }

  async function saveSettings() {
    try {
      const s: any = await invoke("get_settings");
      await invoke("save_settings", {
        newSettings: {
          ...s,
          ai_summary_enabled: summaryEnabled,
          ai_summary_destination: summaryDestination,
          ai_summary_provider: summaryProvider,
          ai_summary_prompt: summaryPrompt,
          hooks,
        },
      });
    } catch (e) { console.error(e); }
  }

  async function onProviderChange() {
    // Re-check if the new provider has an API key
    if (summaryProvider !== "claude_cli") {
      try {
        const s: any = await invoke("get_settings");
        const key = s.api_keys?.[summaryProvider] || "";
        providerKeySet = key.length > 0;
      } catch { providerKeySet = false; }
    }
    saveSettings();
  }

  function addHook() {
    if (!newHookName.trim() || !newHookPath.trim()) return;
    hooks = [...hooks, { id: crypto.randomUUID(), name: newHookName.trim(), script_path: newHookPath.trim(), enabled: true }];
    newHookName = ""; newHookPath = "";
    saveSettings();
  }

  function removeHook(id: string) { hooks = hooks.filter((h) => h.id !== id); saveSettings(); }
  function toggleHook(id: string) { hooks = hooks.map((h) => (h.id === id ? { ...h, enabled: !h.enabled } : h)); saveSettings(); }

  onMount(loadSettings);
</script>

<div class="flex-1 flex flex-col h-full bg-content overflow-auto">
  <div class="px-5 pt-4 pb-3">
    <h1 class="text-lg font-semibold text-text-primary">Automation</h1>
  </div>

  <div class="px-4 pb-4 space-y-3">
    <!-- AI Summary Card -->
    <Card>
      <div class="flex items-start justify-between gap-3">
        <div class="flex items-center gap-2.5">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#4a9eff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/>
          </svg>
          <span class="text-[14px] font-medium text-text-primary">AI Summary</span>
        </div>
        <button
          class="toggle {summaryEnabled ? 'active' : ''}"
          onclick={() => { summaryEnabled = !summaryEnabled; saveSettings(); }}
        ></button>
      </div>

      <p class="text-[12px] text-text-secondary mt-2.5 leading-relaxed">
        Generates a meeting summary with key decisions and action items after each transcription. Written as a separate .md file alongside the transcript.
      </p>

      {#if summaryEnabled}
        <div class="mt-3 pt-3 border-t border-border space-y-3">
          <div class="flex items-center justify-between">
            <span class="text-[13px] text-text-primary">Provider</span>
            <select bind:value={summaryProvider} onchange={onProviderChange} class="w-[180px] text-[12px]">
              <option value="claude_cli">Claude CLI</option>
              <option value="openai">OpenAI (gpt-4o-mini)</option>
              <option value="groq">Groq (llama-3.3-70b)</option>
            </select>
          </div>

          <div class="flex items-center justify-between">
            <span class="text-[13px] text-text-primary">Summary Folder</span>
            <select bind:value={summaryDestination} onchange={saveSettings} class="w-[180px] text-[12px]">
              <option value="same">Same as transcript</option>
              <option value="subfolder">Subfolder (summaries/)</option>
              <option value="fixed">Output folder</option>
            </select>
          </div>

          <div>
            <span class="text-[13px] text-text-primary">Prompt</span>
            <textarea
              bind:value={summaryPrompt}
              onblur={saveSettings}
              rows="3"
              class="w-full text-[12px] mt-1 bg-content border border-border rounded-lg px-3 py-2 text-text-primary resize-none focus:outline-none focus:border-accent"
              placeholder="Enter your summary prompt..."
            ></textarea>
          </div>
        </div>
      {/if}

      <p class="text-[11px] text-text-muted mt-2.5">
        {#if summaryProvider === "claude_cli"}
          {claudeAvailable ? "Claude CLI detected." : "Requires Claude CLI — install from claude.ai/download"}
        {:else}
          {providerKeySet ? "API key configured." : "No API key — add one in Settings > API."}
        {/if}
      </p>
    </Card>

    <!-- Hooks Card -->
    <Card>
      <div class="flex items-center gap-2.5 mb-2.5">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-text-secondary">
          <polyline points="4 17 10 11 4 5"/>
          <line x1="12" x2="20" y1="19" y2="19"/>
        </svg>
        <span class="text-[14px] font-medium text-text-primary">Hooks</span>
      </div>

      <p class="text-[12px] text-text-secondary leading-relaxed">
        Run scripts after each transcription. Scripts receive the transcript path as $1.
      </p>

      <!-- Existing hooks -->
      {#if hooks.length > 0}
        <div class="mt-3 space-y-2">
          {#each hooks as hook (hook.id)}
            <div class="flex items-center gap-2 bg-content rounded-lg px-3 py-2.5">
              <button
                class="toggle {hook.enabled ? 'active' : ''}"
                onclick={() => toggleHook(hook.id)}
                style="transform: scale(0.8);"
              ></button>
              <div class="flex-1 min-w-0">
                <p class="text-[12px] text-text-primary truncate">{hook.name}</p>
                <p class="text-[11px] text-text-muted truncate">{hook.script_path}</p>
              </div>
              <button onclick={() => removeHook(hook.id)} class="text-text-muted hover:text-recording transition-colors">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 6 6 18M6 6l12 12"/></svg>
              </button>
            </div>
          {/each}
        </div>
      {/if}

      <!-- Add Hook -->
      <div class="mt-3 pt-3 border-t border-border">
        <div class="space-y-2">
          <input type="text" bind:value={newHookName} placeholder="Hook name..." class="w-full text-[12px]" />
          <input type="text" bind:value={newHookPath} placeholder="Script path (.bat, .ps1, .sh)..." class="w-full text-[12px]" />
          <button onclick={addHook} class="text-[12px] text-accent hover:text-accent-hover transition-colors font-medium">
            + Add Hook
          </button>
        </div>
      </div>
    </Card>
  </div>
</div>
