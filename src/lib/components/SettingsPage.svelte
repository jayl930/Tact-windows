<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { check } from "@tauri-apps/plugin-updater";
  import { onMount } from "svelte";
  import Card from "./ui/Card.svelte";
  import SettingsRow from "./ui/SettingsRow.svelte";

  let activeSettingsTab = $state("general");

  // General
  let language = $state("en");
  let transcriptionTiming = $state("immediately");
  let diarizationEnabled = $state(false);
  let retentionDays = $state<number | null>(30);
  let launchAtLogin = $state(false);
  let exportAudio = $state(false);
  let enabledLanguages = $state<string[]>(["en", "ko"]);

  const ALL_LANGUAGES: Record<string, string> = {
    en: "English", ko: "한국어", es: "Spanish", fr: "French",
    de: "German", ja: "Japanese", zh: "Chinese", pt: "Portuguese",
    ru: "Russian", ar: "Arabic", hi: "Hindi", it: "Italian",
    nl: "Dutch", sv: "Swedish", pl: "Polish", tr: "Turkish",
  };

  let showLanguagePicker = $state(false);

  // API
  let apiProvider = $state("groq");
  let apiKey = $state("");
  let showApiKey = $state(false);
  let vadEnabled = $state(true);
  let vadThreshold = $state(0.5);
  let testStatus = $state("");
  let isTesting = $state(false);

  // Voice enrollment
  let isEnrollRecording = $state(false);
  let enrolledSpeakers = $state<any[]>([]);

  // Updates
  let updateStatus = $state<"idle" | "checking" | "available" | "latest">("idle");
  let updateError = $state("");

  async function checkForUpdates() {
    updateStatus = "checking";
    try {
      const update = await check();
      if (update) {
        updateStatus = "available";
        await update.downloadAndInstall();
      } else {
        updateStatus = "latest";
        setTimeout(() => { updateStatus = "idle"; }, 3000);
      }
    } catch (e) {
      console.error("Update check failed:", e);
      updateError = String(e);
      updateStatus = "idle";
      setTimeout(() => { updateError = ""; }, 8000);
    }
  }

  async function loadSettings() {
    try {
      const s: any = await invoke("get_settings");
      language = s.language; transcriptionTiming = s.transcription_timing;
      diarizationEnabled = s.diarization_enabled; retentionDays = s.recording_retention_days;
      launchAtLogin = s.launch_at_login; exportAudio = s.export_audio;
      apiProvider = s.api_provider; vadEnabled = s.vad_enabled; vadThreshold = s.vad_threshold;
      enabledLanguages = s.enabled_languages || ["en", "ko"];
    } catch (e) { console.error(e); }
    try { apiKey = await invoke("get_api_key", { provider: apiProvider }); } catch {}
    try { enrolledSpeakers = await invoke("get_enrolled_speakers"); } catch {}
  }

  async function saveSettings() {
    try {
      const s: any = await invoke("get_settings");
      await invoke("save_settings", {
        newSettings: {
          ...s, language, transcription_timing: transcriptionTiming,
          diarization_enabled: diarizationEnabled, recording_retention_days: retentionDays,
          launch_at_login: launchAtLogin, export_audio: exportAudio,
          vad_enabled: vadEnabled, vad_threshold: vadThreshold, api_provider: apiProvider,
          enabled_languages: enabledLanguages,
        },
      });
    } catch (e) { console.error(e); }
  }

  let keySaveStatus = $state("");

  async function saveApiKey() {
    try {
      await invoke("save_api_key", { provider: apiProvider, key: apiKey });
      if (apiKey) { keySaveStatus = "Saved"; setTimeout(() => { keySaveStatus = ""; }, 2000); }
    } catch (e) {
      keySaveStatus = `Failed to save: ${e}`;
      setTimeout(() => { keySaveStatus = ""; }, 5000);
    }
  }

  async function testConnection() {
    if (!apiKey) { testStatus = "Enter an API key first"; return; }
    isTesting = true; testStatus = "Testing...";
    await saveApiKey();
    try { testStatus = await invoke("test_api_connection", { provider: apiProvider, key: apiKey }); }
    catch (e) { testStatus = `${e}`; }
    isTesting = false;
  }

  async function onProviderChange() {
    await saveSettings();
    try { apiKey = await invoke("get_api_key", { provider: apiProvider }); } catch { apiKey = ""; }
    testStatus = "";
  }

  async function startEnrollRecording() {
    try { await invoke("start_recording"); isEnrollRecording = true; } catch (e) { console.error(e); }
  }

  async function stopEnrollRecording() {
    try {
      const speaker: any = await invoke("enroll_speaker", { name: "Me" });
      enrolledSpeakers = [...enrolledSpeakers, speaker];
      isEnrollRecording = false;
    } catch (e) { console.error(e); isEnrollRecording = false; }
  }

  async function removeSpeaker(id: string) {
    try { await invoke("remove_enrolled_speaker", { id }); enrolledSpeakers = enrolledSpeakers.filter((s) => s.id !== id); } catch (e) { console.error(e); }
  }

  onMount(loadSettings);
</script>

<div class="flex-1 flex flex-col h-full bg-content overflow-auto">
  <!-- Tab bar -->
  <div class="flex gap-1 px-4 pt-4 pb-2">
    <button
      class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-[13px] font-medium transition-colors
        {activeSettingsTab === 'general' ? 'bg-accent-muted text-text-primary' : 'text-text-muted hover:text-text-secondary'}"
      onclick={() => (activeSettingsTab = "general")}
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/>
        <circle cx="12" cy="12" r="3"/>
      </svg>
      General
    </button>
    <button
      class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-[13px] font-medium transition-colors
        {activeSettingsTab === 'api' ? 'bg-accent-muted text-text-primary' : 'text-text-muted hover:text-text-secondary'}"
      onclick={() => (activeSettingsTab = "api")}
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <rect width="18" height="11" x="3" y="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>
      </svg>
      API
    </button>
  </div>

  <div class="px-4 pb-4 space-y-3">
    {#if activeSettingsTab === "general"}
      <!-- General Card -->
      <Card>
        <h3 class="text-[12px] font-semibold text-text-muted uppercase tracking-wider mb-1">General</h3>

        <SettingsRow label="Default Language">
          <select bind:value={language} onchange={saveSettings} class="w-[120px] text-[12px]">
            {#each enabledLanguages as code}
              <option value={code}>{ALL_LANGUAGES[code] || code}</option>
            {/each}
          </select>
        </SettingsRow>

        <div class="border-t border-border"></div>

        <SettingsRow label="Languages" subtitle="Choose which languages appear in dropdowns">
          <button
            onclick={() => { showLanguagePicker = !showLanguagePicker; }}
            class="text-[12px] text-accent hover:text-accent-hover font-medium transition-colors"
          >
            {showLanguagePicker ? "Done" : "Edit"}
          </button>
        </SettingsRow>

        {#if showLanguagePicker}
          <div class="grid grid-cols-2 gap-1.5 pt-1 pb-2">
            {#each Object.entries(ALL_LANGUAGES) as [code, name]}
              <label class="flex items-center gap-2 px-2 py-1.5 rounded-lg hover:bg-surface-hover cursor-pointer transition-colors">
                <input
                  type="checkbox"
                  checked={enabledLanguages.includes(code)}
                  onchange={() => {
                    if (enabledLanguages.includes(code)) {
                      if (enabledLanguages.length > 1) {
                        enabledLanguages = enabledLanguages.filter(l => l !== code);
                      }
                    } else {
                      enabledLanguages = [...enabledLanguages, code];
                    }
                    saveSettings();
                  }}
                  class="accent-accent w-3.5 h-3.5"
                />
                <span class="text-[11px] text-text-primary">{name}</span>
              </label>
            {/each}
          </div>
        {/if}

        <div class="border-t border-border"></div>

        <SettingsRow label="Launch at Login">
          <button class="toggle {launchAtLogin ? 'active' : ''}"
            onclick={() => { launchAtLogin = !launchAtLogin; saveSettings(); }}></button>
        </SettingsRow>

        <div class="border-t border-border"></div>

        <SettingsRow label="Shortcut">
          <span class="text-[12px] text-text-muted bg-content px-2 py-1 rounded-md font-mono">Ctrl+Shift+R</span>
        </SettingsRow>

        <div class="border-t border-border"></div>

        <SettingsRow label="Updates">
          <button
            onclick={checkForUpdates}
            disabled={updateStatus === "checking"}
            class="text-[12px] text-accent hover:text-accent-hover font-medium transition-colors disabled:opacity-50"
          >
            {updateStatus === "checking" ? "Checking..." : updateStatus === "available" ? "Update Available — Install" : "Check for Updates"}
          </button>
        </SettingsRow>

        {#if updateStatus === "latest"}
          <p class="text-[11px] text-text-muted pb-1">You're on the latest version.</p>
        {/if}
        {#if updateError}
          <p class="text-[11px] text-recording pb-1">{updateError}</p>
        {/if}

        <div class="border-t border-border"></div>

        <SettingsRow label="Recording Retention">
          <select bind:value={retentionDays} onchange={saveSettings} class="w-[100px] text-[12px]">
            <option value={7}>7 days</option>
            <option value={30}>30 days</option>
            <option value={90}>90 days</option>
            <option value={null}>Forever</option>
          </select>
        </SettingsRow>

        <div class="border-t border-border"></div>

        <SettingsRow label="Export Audio">
          <button class="toggle {exportAudio ? 'active' : ''}"
            onclick={() => { exportAudio = !exportAudio; saveSettings(); }}></button>
        </SettingsRow>
      </Card>

      <!-- Transcription Card -->
      <Card>
        <h3 class="text-[12px] font-semibold text-text-muted uppercase tracking-wider mb-1">Transcription</h3>

        <SettingsRow label="Diarization" subtitle="Identify speakers in transcripts">
          <button class="toggle {diarizationEnabled ? 'active' : ''}"
            onclick={() => { diarizationEnabled = !diarizationEnabled; saveSettings(); }}></button>
        </SettingsRow>

        {#if diarizationEnabled}
          <div class="border-t border-border"></div>

          <SettingsRow label="My Voice" subtitle="Record 10-30 seconds of your speech to identify you">
            {#if isEnrollRecording}
              <button onclick={stopEnrollRecording}
                class="text-[12px] text-recording hover:text-red-400 font-medium transition-colors">
                Stop & Save
              </button>
            {:else}
              <button onclick={startEnrollRecording}
                class="text-[12px] text-accent hover:text-accent-hover font-medium transition-colors">
                Record My Voice
              </button>
            {/if}
          </SettingsRow>

          {#if enrolledSpeakers.length > 0}
            {#each enrolledSpeakers as speaker (speaker.id)}
              <div class="flex items-center justify-between py-1.5 pl-4">
                <span class="text-[12px] text-text-secondary">{speaker.name}</span>
                <button onclick={() => removeSpeaker(speaker.id)}
                  class="text-[11px] text-text-muted hover:text-recording transition-colors">Remove</button>
              </div>
            {/each}
          {/if}
        {/if}

        <div class="border-t border-border"></div>

        <SettingsRow label="Transcription Timing">
          <select bind:value={transcriptionTiming} onchange={saveSettings} class="w-[120px] text-[12px]">
            <option value="immediately">Immediately</option>
            <option value="on_return">On Return</option>
            <option value="manual">Manual</option>
          </select>
        </SettingsRow>
      </Card>

    {:else if activeSettingsTab === "api"}
      <!-- Provider Card -->
      <Card>
        <h3 class="text-[12px] font-semibold text-text-muted uppercase tracking-wider mb-1">Provider</h3>

        <SettingsRow label="API Provider">
          <select bind:value={apiProvider} onchange={onProviderChange} class="w-[160px] text-[12px]">
            <option value="groq">Groq (Whisper Turbo)</option>
            <option value="openai">OpenAI (Whisper-1)</option>
          </select>
        </SettingsRow>

        <div class="border-t border-border"></div>

        <div class="py-2.5 space-y-2">
          <label class="text-[13px] text-text-primary">API Key</label>
          <div class="flex gap-2">
            <input
              type={showApiKey ? "text" : "password"}
              bind:value={apiKey}
              onblur={saveApiKey}
              onchange={saveApiKey}
              placeholder="sk-..."
              class="flex-1 text-[12px]"
            />
            <button onclick={() => (showApiKey = !showApiKey)}
              class="px-2.5 py-1.5 text-[11px] text-text-muted bg-content border border-border rounded-lg hover:text-text-secondary transition-colors">
              {showApiKey ? "Hide" : "Show"}
            </button>
          </div>
          {#if keySaveStatus}
            <p class="text-[11px] {keySaveStatus === 'Saved' ? 'text-green-400' : 'text-recording'}">{keySaveStatus}</p>
          {/if}
        </div>

        <button onclick={testConnection} disabled={isTesting}
          class="w-full py-2 text-[12px] font-medium bg-accent/15 text-accent hover:bg-accent/25 rounded-lg transition-colors disabled:opacity-50">
          {isTesting ? "Testing..." : "Test Connection"}
        </button>
        {#if testStatus}
          <p class="text-[11px] mt-2 {testStatus.includes('successful') ? 'text-green-400' : 'text-recording'}">
            {testStatus}
          </p>
        {/if}
      </Card>

      <!-- VAD Card -->
      <Card>
        <h3 class="text-[12px] font-semibold text-text-muted uppercase tracking-wider mb-1">Voice Activity Detection</h3>

        <SettingsRow label="Trim Silence" subtitle="Remove silence before sending to API to reduce cost">
          <button class="toggle {vadEnabled ? 'active' : ''}"
            onclick={() => { vadEnabled = !vadEnabled; saveSettings(); }}></button>
        </SettingsRow>

        {#if vadEnabled}
          <div class="border-t border-border"></div>
          <SettingsRow label="Threshold" subtitle={vadThreshold.toFixed(2)}>
            <input type="range" min="0" max="1" step="0.05" bind:value={vadThreshold}
              onchange={saveSettings}
              class="w-[100px] accent-accent" />
          </SettingsRow>
        {/if}
      </Card>
    {/if}
  </div>
</div>
