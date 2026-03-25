<script lang="ts">
  import { onMount } from "svelte";
  import SidebarNavigation from "$lib/components/SidebarNavigation.svelte";
  import RecordPage from "$lib/components/RecordPage.svelte";
  import ScheduledPage from "$lib/components/ScheduledPage.svelte";
  import UploadPage from "$lib/components/UploadPage.svelte";
  import AutomationPage from "$lib/components/AutomationPage.svelte";
  import SettingsPage from "$lib/components/SettingsPage.svelte";
  import { createSettingsStore } from "$lib/stores/settings.svelte";
  import { createAppStateStore } from "$lib/stores/appState.svelte";

  const settingsStore = createSettingsStore();
  const appState = createAppStateStore();

  let activeTab = $state("record");

  function handleTabChange(tab: string) {
    activeTab = tab;
  }

  onMount(() => {
    settingsStore.load();
    appState.load();
  });
</script>

<main class="flex h-screen w-screen overflow-hidden bg-content text-text-primary">
  <SidebarNavigation {activeTab} onTabChange={handleTabChange} />

  <div class="flex-1 overflow-auto">
    {#if activeTab === "record"}
      <RecordPage />
    {:else if activeTab === "scheduled"}
      <ScheduledPage />
    {:else if activeTab === "upload"}
      <UploadPage />
    {:else if activeTab === "automation"}
      <AutomationPage />
    {:else if activeTab === "settings"}
      <SettingsPage />
    {/if}
  </div>
</main>
