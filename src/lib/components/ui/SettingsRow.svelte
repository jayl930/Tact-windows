<script lang="ts">
  let { label, subtitle, children }: { label: string; subtitle?: string; children: any } = $props();
</script>

<div class="flex items-center justify-between py-2.5 min-h-[36px]">
  <div class="flex-1 min-w-0 mr-3">
    <span class="text-[13px] text-text-primary">{label}</span>
    {#if subtitle}
      <p class="text-[11px] text-text-muted mt-0.5">{subtitle}</p>
    {/if}
  </div>
  <div class="flex-shrink-0">
    {@render children()}
  </div>
</div>
