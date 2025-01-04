import { writable } from 'svelte/store';

type Notification = {
    message: string;
    type: 'default' | 'destructive' | 'success';
    id: number;
}

function createNotificationStore() {
    const { subscribe, update } = writable<Notification[]>([]);

    return {
        subscribe,
        add: (message: string, type: 'default' | 'destructive' | 'success' = 'default') => {
            const id = Date.now();

            update(notifications => [
                ...notifications,
                {
                    id,
                    message,
                    type
                }
            ]);

            // Remove this specific notification after 5 seconds using the saved id
            setTimeout(() => {
                update(notifications =>
                    notifications.filter(n => n.id !== id)
                );
            }, 5000);
        },
        remove: (id: number) => {
            update(notifications =>
                notifications.filter(n => n.id !== id)
            );
        }
    };
}

export const notifications = createNotificationStore();
