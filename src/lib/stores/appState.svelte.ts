import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface AppStateData {
  is_recording: boolean;
  is_transcribing: boolean;
  current_duration: number;
  active_tab: string;
}

export function createAppStateStore() {
  let state = $state<AppStateData>({
    is_recording: false,
    is_transcribing: false,
    current_duration: 0,
    active_tab: "record",
  });

  listen<AppStateData>("app-state-changed", (event) => {
    Object.assign(state, event.payload);
  });

  async function load() {
    try {
      const s = await invoke<AppStateData>("get_app_state");
      Object.assign(state, s);
    } catch (e) {
      console.error("Failed to load app state:", e);
    }
  }

  return {
    get value() {
      return state;
    },
    load,
  };
}
