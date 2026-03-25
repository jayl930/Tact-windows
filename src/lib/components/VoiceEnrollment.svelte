<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  interface EnrolledSpeaker {
    id: string;
    name: string;
    audio_path: string;
    created_at: string;
  }

  let speakers = $state<EnrolledSpeaker[]>([]);
  let newName = $state("");
  let isRecording = $state(false);
  let statusMessage = $state("");

  async function loadSpeakers() {
    try {
      speakers = await invoke("get_enrolled_speakers");
    } catch (e) {
      console.error("Failed to load speakers:", e);
    }
  }

  async function startEnrollment() {
    if (!newName.trim()) {
      statusMessage = "Enter a name first";
      return;
    }
    try {
      await invoke("start_recording");
      isRecording = true;
      statusMessage = "Recording... Speak for 5-10 seconds, then click Stop";
    } catch (e) {
      statusMessage = `Error: ${e}`;
    }
  }

  async function stopEnrollment() {
    try {
      const speaker: EnrolledSpeaker = await invoke("enroll_speaker", {
        name: newName.trim(),
      });
      speakers = [...speakers, speaker];
      newName = "";
      isRecording = false;
      statusMessage = `Enrolled "${speaker.name}"`;
    } catch (e) {
      statusMessage = `Error: ${e}`;
      isRecording = false;
    }
  }

  async function removeSpeaker(id: string) {
    try {
      await invoke("remove_enrolled_speaker", { id });
      speakers = speakers.filter((s) => s.id !== id);
    } catch (e) {
      statusMessage = `Error: ${e}`;
    }
  }

  onMount(loadSpeakers);
</script>

<div class="space-y-4">
  <h3 class="text-xs font-medium text-text-primary">Voice Enrollment</h3>
  <p class="text-xs text-text-secondary">
    Enroll speakers so Tact can label them in transcripts.
  </p>

  <!-- Enrolled speakers list -->
  {#if speakers.length > 0}
    <div class="space-y-2">
      {#each speakers as speaker (speaker.id)}
        <div class="flex items-center justify-between bg-surface rounded px-3 py-2">
          <span class="text-xs text-text-primary">{speaker.name}</span>
          <button
            onclick={() => removeSpeaker(speaker.id)}
            class="text-xs text-text-secondary hover:text-recording"
          >
            Remove
          </button>
        </div>
      {/each}
    </div>
  {:else}
    <p class="text-xs text-text-secondary italic">No speakers enrolled</p>
  {/if}

  <!-- Enroll new speaker -->
  <div class="space-y-2">
    <input
      type="text"
      bind:value={newName}
      placeholder="Speaker name..."
      class="w-full bg-surface border border-border rounded px-2 py-1.5 text-sm text-text-primary"
    />
    {#if isRecording}
      <button
        onclick={stopEnrollment}
        class="w-full py-1.5 text-xs bg-recording hover:bg-red-600 text-white rounded"
      >
        Stop & Save
      </button>
    {:else}
      <button
        onclick={startEnrollment}
        class="w-full py-1.5 text-xs bg-accent hover:bg-accent-active text-text-primary rounded"
      >
        Record Voice Sample
      </button>
    {/if}
  </div>

  {#if statusMessage}
    <p class="text-xs text-text-secondary">{statusMessage}</p>
  {/if}
</div>
