import { writable } from 'svelte/store';

export const toasts = writable([]);

const MAX_TOASTS = 5;

export function addToast(message, type = 'error') {
  const id = Date.now();
  toasts.update(t => {
    const next = [...t, { id, message, type }];
    // 超过上限时移除最早的
    return next.length > MAX_TOASTS ? next.slice(-MAX_TOASTS) : next;
  });
  setTimeout(() => {
    toasts.update(t => t.filter(toast => toast.id !== id));
  }, 4000);
}

export function removeToast(id) {
  toasts.update(t => t.filter(toast => toast.id !== id));
}
