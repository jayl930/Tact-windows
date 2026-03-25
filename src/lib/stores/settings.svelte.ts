import { invoke } from "@tauri-apps/api/core";

export interface TactSettings {
  language: string;
  transcription_timing: string;
  diarization_enabled: boolean;
  recording_retention_days: number | null;
  launch_at_login: boolean;
  export_audio: boolean;
  vad_enabled: boolean;
  vad_threshold: number;
  api_provider: string;
  output_folder: string | null;
  favorite_folders: string[];
  recent_folders: string[];
  ai_summary_enabled: boolean;
  ai_summary_destination: string;
  hooks: HookConfig[];
  enabled_languages: string[];
}

export interface HookConfig {
  id: string;
  name: string;
  script_path: string;
  enabled: boolean;
}

const defaultSettings: TactSettings = {
  language: "en",
  transcription_timing: "immediately",
  diarization_enabled: false,
  recording_retention_days: 30,
  launch_at_login: false,
  export_audio: false,
  vad_enabled: true,
  vad_threshold: 0.5,
  api_provider: "groq",
  output_folder: null,
  favorite_folders: [],
  recent_folders: [],
  ai_summary_enabled: false,
  ai_summary_destination: "same",
  hooks: [],
  enabled_languages: ["en", "ko"],
};

export function createSettingsStore() {
  let settings = $state<TactSettings>({ ...defaultSettings });
  let loaded = $state(false);

  async function load() {
    try {
      const s = await invoke<TactSettings>("get_settings");
      Object.assign(settings, s);
      loaded = true;
    } catch (e) {
      console.error("Failed to load settings:", e);
    }
  }

  async function save() {
    try {
      await invoke("save_settings", { settings });
    } catch (e) {
      console.error("Failed to save settings:", e);
    }
  }

  return {
    get value() {
      return settings;
    },
    get loaded() {
      return loaded;
    },
    load,
    save,
  };
}
