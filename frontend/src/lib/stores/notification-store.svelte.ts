type NotificationType = 'default' | 'destructive' | 'success';

interface Notification {
	id: number;
	message: string;
	type: NotificationType;
}

export function createNotificationStore() {
	let items = $state<Notification[]>([]);

	return {
		get list() {
			return items;
		},
		add(message: string, type: NotificationType = 'default') {
			const id = Date.now();
			items = [...items, { id, message, type }];
			setTimeout(() => {
				items = items.filter((n) => n.id !== id);
			}, 5000);
		},
		remove(id: number) {
			items = items.filter((n) => n.id !== id);
		}
	};
}

export const notifications = createNotificationStore();
