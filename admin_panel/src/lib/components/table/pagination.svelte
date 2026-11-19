<script lang="ts">
	import { Button } from '#lib/components/ui/button/index.ts';

	interface Props {
		currentPage: number;
		totalPages: number;
		totalItems: number;
		itemsPerPage?: number;
		onPageChange: (page: number) => void;
	}

	let { currentPage, totalPages, totalItems, itemsPerPage = 10, onPageChange }: Props = $props();

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
	<div class="text-muted-foreground text-sm">
		Showing {startItem} to {endItem} of {totalItems} entries
	</div>
	<div class="flex gap-2">
		<Button
			variant="outline"
			size="sm"
			onclick={() => onPageChange(0)}
			disabled={currentPage === 0}
		>
			First
		</Button>
		<Button variant="outline" size="sm" onclick={previousPage} disabled={currentPage === 0}>
			Previous
		</Button>
		<Button variant="outline" size="sm" onclick={nextPage} disabled={currentPage >= totalPages - 1}>
			Next
		</Button>
		<Button
			variant="outline"
			size="sm"
			onclick={() => onPageChange(totalPages - 1)}
			disabled={currentPage >= totalPages - 1}
		>
			Last
		</Button>
	</div>
</div>
