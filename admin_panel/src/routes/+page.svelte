<script lang="ts">
  import CSVTable from '$lib/components/CSVTable.svelte';
</script>

<div class="container mx-auto py-8">
  <header class="mb-8">
    <h1 class="text-3xl font-bold">CSV Character Manager</h1>
    <p class="text-muted-foreground mt-2">Upload and manage your character CSV data</p>
  </header>

  <CSVTable />
</div>
