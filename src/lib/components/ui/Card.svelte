<script lang="ts">
  let { children }: { children: any } = $props();
</script>

<div class="rounded-xl border border-white/[0.06] p-4 shadow-[0_1px_4px_rgba(0,0,0,0.15)]" style="background: rgba(57, 57, 57, 0.55); backdrop-filter: blur(12px);">
  {@render children()}
</div>
