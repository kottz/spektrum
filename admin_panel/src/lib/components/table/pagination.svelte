<script lang="ts">
	import { Button } from '$lib/components/ui/button';

	let {
		currentPage,
		totalPages,
		totalItems,
		itemsPerPage = 10,
		onPageChange
	} = $props<{
		currentPage: number;
		totalPages: number;
		totalItems: number;
		itemsPerPage?: number;
		onPageChange: (page: number) => void;
	}>();

	function nextPage() {
		if (currentPage < totalPages - 1) {
			onPageChange(currentPage + 1);
		}
	}

	function previousPage() {
		if (currentPage > 0) {
			onPageChange(currentPage - 1);
		}
	}

	const startItem = $derived(currentPage * itemsPerPage + 1);
	const endItem = $derived(Math.min((currentPage + 1) * itemsPerPage, totalItems));
</script>

<div class="flex items-center justify-between border-t p-4">
	<div class="text-sm text-muted-foreground">
		Showing {startItem} to {endItem} of {totalItems} entries
	</div>
	<div class="flex gap-2">
		<Button
			variant="outline"
			size="sm"
			on:click={() => onPageChange(0)}
			disabled={currentPage === 0}
		>
			First
		</Button>
		<Button variant="outline" size="sm" on:click={previousPage} disabled={currentPage === 0}>
			Previous
		</Button>
		<Button
			variant="outline"
			size="sm"
			on:click={nextPage}
			disabled={currentPage >= totalPages - 1}
		>
			Next
		</Button>
		<Button
			variant="outline"
			size="sm"
			on:click={() => onPageChange(totalPages - 1)}
			disabled={currentPage >= totalPages - 1}
		>
			Last
		</Button>
	</div>
</div>
