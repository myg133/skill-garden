import { writable } from 'svelte/store';

export const toasts = writable([]);

export function addToast(message, type = 'error') {
  const id = Date.now();
  toasts.update(t => [...t, { id, message, type }]);
  setTimeout(() => {
    toasts.update(t => t.filter(toast => toast.id !== id));
  }, 4000);
}
