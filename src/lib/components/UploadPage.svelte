<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import Card from "./ui/Card.svelte";
  import SettingsRow from "./ui/SettingsRow.svelte";

  let selectedFile = $state<string | null>(null);
  let fileName = $state("");
  let isTranscribing = $state(false);
  let statusMessage = $state("");

  // Settings for upload transcription
  let language = $state("en");
  let transcriptionTiming = $state("immediately");
  let apiProvider = $state("groq");
  let diarizationEnabled = $state(false);
  let enabledLanguages = $state<string[]>(["en", "ko"]);

  const ALL_LANGUAGES: Record<string, string> = {
    en: "English", ko: "한국어", es: "Spanish", fr: "French",
    de: "German", ja: "Japanese", zh: "Chinese", pt: "Portuguese",
    ru: "Russian", ar: "Arabic", hi: "Hindi", it: "Italian",
  };

  async function loadSettings() {
    try {
      const s: any = await invoke("get_settings");
      language = s.language;
      transcriptionTiming = s.transcription_timing;
      apiProvider = s.api_provider;
      diarizationEnabled = s.diarization_enabled;
      enabledLanguages = s.enabled_languages || ["en", "ko"];
    } catch (e) { console.error(e); }
  }

  async function saveSetting(field: string, value: any) {
    try {
      const s: any = await invoke("get_settings");
      (s as any)[field] = value;
      await invoke("save_settings", { newSettings: s });
    } catch (e) { console.error(e); }
  }

  async function pickFile() {
    try {
      const result = await open({
        multiple: false,
        filters: [{ name: "Audio", extensions: ["m4a", "wav", "mp3", "ogg", "opus", "flac"] }],
      });
      if (result) {
        selectedFile = result as string;
        fileName = selectedFile.split(/[\\/]/).pop() || selectedFile;
        statusMessage = "";
      }
    } catch (e) { statusMessage = `Error: ${e}`; }
  }

  async function transcribe() {
    if (!selectedFile) return;
    isTranscribing = true;
    statusMessage = "";
    try {
      const result: any = await invoke("transcribe_file_cmd", { audioPath: selectedFile });
      statusMessage = `Done! ${result.segment_count} segments, ${result.duration?.toFixed(0)}s`;
      selectedFile = null;
      fileName = "";
    } catch (e) { statusMessage = `Error: ${e}`; }
    isTranscribing = false;
  }

  function providerLabel(p: string): string {
    return p === "openai" ? "OpenAI Whisper" : "Groq Turbo";
  }

  onMount(() => {
    loadSettings();
    const unlisten = listen("transcription-complete", () => { isTranscribing = false; });
    return () => { unlisten.then((f) => f()); };
  });
</script>

<div class="flex-1 flex flex-col h-full bg-content overflow-auto">
  <!-- Header -->
  <div class="px-5 pt-4 pb-3">
    <h1 class="text-lg font-semibold text-text-primary">Upload Audio</h1>
  </div>

  <div class="flex-1 flex flex-col px-4 pb-4">
    <!-- File selector button -->
    <button
      onclick={pickFile}
      disabled={isTranscribing}
      class="w-full flex items-center gap-3 bg-surface hover:bg-surface-hover border border-border
        rounded-xl px-4 py-3 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-text-muted flex-shrink-0">
        <circle cx="12" cy="12" r="10"/><line x1="12" x2="12" y1="8" y2="16"/><line x1="8" x2="16" y1="12" y2="12"/>
      </svg>
      <span class="text-[13px] {selectedFile ? 'text-text-primary' : 'text-text-secondary'}">
        {selectedFile ? fileName : "Select audio file (.m4a, .wav, .mp3)"}
      </span>
    </button>

    <!-- Settings card -->
    <div class="mt-3">
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
          <span class="inline-flex items-center gap-1.5 text-[12px] text-accent bg-accent/10 px-2.5 py-1 rounded-md">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/>
            </svg>
            transcripts
          </span>
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
            aria-label="Toggle identify me"
            class="toggle {diarizationEnabled ? 'active' : ''}"
            onclick={() => { diarizationEnabled = !diarizationEnabled; saveSetting("diarization_enabled", diarizationEnabled); }}
          ></button>
        </SettingsRow>
      </Card>
    </div>

    <!-- Status message -->
    {#if statusMessage}
      <p class="text-[12px] text-text-secondary text-center mt-3">{statusMessage}</p>
    {/if}

    <!-- Spacer pushes transcribe button to bottom -->
    <div class="flex-1"></div>

    <!-- Transcribe button at bottom -->
    <button
      onclick={transcribe}
      disabled={isTranscribing || !selectedFile}
      class="w-full flex items-center justify-center gap-2 py-3 rounded-xl text-[14px] font-medium transition-all
        {selectedFile && !isTranscribing
        ? 'bg-surface hover:bg-surface-hover text-text-primary border border-border'
        : 'bg-surface/50 text-text-muted border border-border/50 cursor-not-allowed'}"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <rect x="2" y="6" width="4" height="12" rx="1"/><rect x="7" y="3" width="4" height="18" rx="1"/><rect x="12" y="8" width="4" height="8" rx="1"/><rect x="17" y="5" width="4" height="14" rx="1"/>
      </svg>
      {isTranscribing ? "Transcribing..." : "Transcribe"}
    </button>
  </div>
</div>
