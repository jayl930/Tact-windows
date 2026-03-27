<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount, onDestroy } from "svelte";
  import Card from "./ui/Card.svelte";
  import SettingsRow from "./ui/SettingsRow.svelte";

  let isRecording = $state(false);
  let isTranscribing = $state(false);
  let duration = $state(0);
  let statusMessage = $state("");
  let durationInterval: ReturnType<typeof setInterval> | null = null;

  // Inline settings
  let language = $state("en");
  let transcriptionTiming = $state("immediately");
  let apiProvider = $state("groq");
  let diarizationEnabled = $state(false);
  let enabledLanguages = $state<string[]>(["en", "ko"]);
  let outputFolder = $state<string | null>(null);

  const ALL_LANGUAGES: Record<string, string> = {
    en: "English", ko: "한국어", es: "Spanish", fr: "French",
    de: "German", ja: "Japanese", zh: "Chinese", pt: "Portuguese",
    ru: "Russian", ar: "Arabic", hi: "Hindi", it: "Italian",
  };

  function formatDuration(secs: number): string {
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    const s = Math.floor(secs % 60);
    return `${h.toString().padStart(2, "0")}:${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
  }

  function todayDate(): string {
    return new Date().toLocaleDateString("en-US", { weekday: "long", month: "short", day: "numeric" });
  }

  async function loadSettings() {
    try {
      const s: any = await invoke("get_settings");
      language = s.language;
      transcriptionTiming = s.transcription_timing;
      apiProvider = s.api_provider;
      diarizationEnabled = s.diarization_enabled;
      enabledLanguages = s.enabled_languages || ["en", "ko"];
      outputFolder = s.output_folder || null;
    } catch (e) { console.error(e); }
  }

  async function saveSetting(field: string, value: any) {
    try {
      const s: any = await invoke("get_settings");
      (s as any)[field] = value;
      await invoke("save_settings", { newSettings: s });
    } catch (e) { console.error(e); }
  }

  async function toggleRecording() {
    if (isRecording) {
      try {
        statusMessage = "Stopping...";
        const result: any = await invoke("stop_recording");
        isRecording = false;
        duration = 0;
        if (durationInterval) { clearInterval(durationInterval); durationInterval = null; }
        if (result.transcribed) statusMessage = "Transcript saved!";
        else if (result.reason === "no_api_key") statusMessage = "Saved. Add API key in Settings.";
        else statusMessage = "Recording saved.";
      } catch (e) { statusMessage = `Error: ${e}`; }
    } else {
      try {
        await invoke("start_recording");
        isRecording = true;
        statusMessage = "";
        duration = 0;
        durationInterval = setInterval(async () => {
          try { duration = await invoke("get_recording_duration"); } catch {}
        }, 200);
      } catch (e) { statusMessage = `Error: ${e}`; }
    }
  }

  function folderName(path: string | null): string {
    if (!path) return "transcripts";
    return path.split(/[\\/]/).pop() || path;
  }

  async function pickOutputFolder() {
    try {
      const result = await open({ directory: true, multiple: false });
      if (result) {
        const folder = result as string;
        await invoke("set_output_folder", { folder });
        outputFolder = folder;
      }
    } catch (e) { console.error(e); }
  }

  function providerLabel(p: string): string {
    return p === "openai" ? "OpenAI Whisper" : "Groq Turbo";
  }

  onMount(() => {
    loadSettings();
    const unlistens = [
      listen("hotkey-toggle-record", () => toggleRecording()),
      listen("transcription-complete", () => { statusMessage = "Transcript saved!"; isTranscribing = false; }),
      listen<string>("transcription-error", (e) => { statusMessage = `Error: ${e.payload}`; isTranscribing = false; }),
      listen("transcription-started", () => { statusMessage = "Transcribing..."; isTranscribing = true; }),
      listen<any>("app-state-changed", (e) => { isTranscribing = e.payload.is_transcribing; }),
    ];
    return () => { unlistens.forEach((u) => u.then((f) => f())); };
  });

  onDestroy(() => { if (durationInterval) clearInterval(durationInterval); });
</script>

<div class="flex-1 flex flex-col h-full bg-content overflow-hidden">
  <!-- Header -->
  <div class="px-5 pt-4 pb-1">
    <h1 class="text-[18px] font-bold text-text-primary tracking-tight">Tact</h1>
    <p class="text-[12px] text-text-muted mt-0.5">{todayDate()}</p>
  </div>

  <!-- Record area -->
  <div class="flex-1 flex flex-col items-center justify-center gap-2">
    <!-- Duration (shown when recording) -->
    {#if isRecording}
      <div class="text-[28px] font-mono text-text-primary tabular-nums tracking-[0.15em] mb-1">
        {formatDuration(duration)}
      </div>
    {/if}

    <!-- Concentric circle record button -->
    <button
      onclick={toggleRecording}
      disabled={isTranscribing}
      aria-label={isRecording ? "Stop recording" : "Start recording"}
      class="relative w-[80px] h-[80px] rounded-full flex items-center justify-center transition-all duration-200
        {isTranscribing ? 'opacity-40 cursor-not-allowed' : 'hover:scale-[1.04] active:scale-[0.96]'}"
      style={isRecording
        ? "filter: drop-shadow(0 0 20px rgba(231, 76, 60, 0.35));"
        : "filter: drop-shadow(0 0 20px rgba(74, 158, 255, 0.25));"}
    >
      <!-- Outer glow ring -->
      <div class="absolute inset-0 rounded-full transition-colors duration-300
        {isRecording ? 'bg-recording/20' : 'bg-accent/15'}"></div>
      <!-- Middle ring -->
      <div class="absolute inset-[10px] rounded-full transition-colors duration-300
        {isRecording ? 'bg-recording/40' : 'bg-accent/30'}
        {isRecording ? 'shadow-[inset_0_0_12px_rgba(231,76,60,0.3)]' : 'shadow-[inset_0_0_12px_rgba(74,158,255,0.2)]'}"></div>
      <!-- Inner solid circle -->
      <div class="absolute inset-[20px] rounded-full flex items-center justify-center transition-colors duration-300
        {isRecording ? 'bg-recording' : 'bg-accent'}">
        {#if isRecording}
          <div class="w-[14px] h-[14px] bg-white rounded-[3px]"></div>
        {:else}
          <div class="w-[10px] h-[10px] bg-white rounded-full opacity-90"></div>
        {/if}
      </div>
    </button>

    <!-- Label -->
    <div class="text-center mt-1">
      <p class="text-[14px] font-semibold text-text-primary">
        {#if isRecording}Stop Recording{:else if isTranscribing}Transcribing...{:else}Start Recording{/if}
      </p>
      <p class="text-[11px] text-text-muted mt-1 font-mono tracking-wider">
        {#if statusMessage}{statusMessage}{:else}Ctrl+Shift+R{/if}
      </p>
    </div>
  </div>

  <!-- Inline settings card -->
  <div class="px-4 pb-4">
    <Card>
      <SettingsRow label="Language">
        <select bind:value={language} onchange={() => saveSetting("language", language)} class="w-[120px] text-[12px]">
          {#each enabledLanguages as code}
            <option value={code}>{ALL_LANGUAGES[code] || code}</option>
          {/each}
        </select>
      </SettingsRow>

      <div class="border-t border-border"></div>

      <SettingsRow label="Destination">
        <button
          onclick={pickOutputFolder}
          class="inline-flex items-center gap-1.5 text-[12px] text-accent bg-accent/10 px-2.5 py-1 rounded-md hover:bg-accent/20 transition-colors cursor-pointer"
          title={outputFolder || "Default: transcripts folder"}
        >
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/>
          </svg>
          {folderName(outputFolder)}
        </button>
      </SettingsRow>

      <div class="border-t border-border"></div>

      <SettingsRow label="Provider">
        <span class="text-[12px] text-text-secondary">{providerLabel(apiProvider)}</span>
      </SettingsRow>

      <div class="border-t border-border"></div>

      <SettingsRow label="Transcribe">
        <select bind:value={transcriptionTiming} onchange={() => saveSetting("transcription_timing", transcriptionTiming)} class="w-[120px] text-[12px]">
          <option value="immediately">Immediately</option>
          <option value="on_return">On Return</option>
          <option value="manual">Manual</option>
        </select>
      </SettingsRow>

      <div class="border-t border-border"></div>

      <SettingsRow label="Identify Me">
        <button
          aria-label="Toggle speaker identification"
          class="toggle {diarizationEnabled ? 'active' : ''}"
          onclick={() => { diarizationEnabled = !diarizationEnabled; saveSetting("diarization_enabled", diarizationEnabled); }}
        ></button>
      </SettingsRow>
    </Card>
  </div>
</div>
