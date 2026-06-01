import { writable, derived } from 'svelte/store';

const TOKEN_KEY = 'admin_token';
const USERNAME_KEY = 'admin_username';
const NAV_KEY = 'admin_selected_nav';

function createAuthStore() {
  const { subscribe, set, update } = writable({
    token: localStorage.getItem(TOKEN_KEY) || null,
    username: localStorage.getItem(USERNAME_KEY) || null,
    loading: false,
    error: null,
  });

  return {
    subscribe,

    login(token, username) {
      localStorage.setItem(TOKEN_KEY, token);
      localStorage.setItem(USERNAME_KEY, username);
      set({ token, username, loading: false, error: null });
    },

    logout() {
      localStorage.removeItem(TOKEN_KEY);
      localStorage.removeItem(USERNAME_KEY);
      localStorage.removeItem(NAV_KEY);
      set({ token: null, username: null, loading: false, error: null });
    },

    setLoading(loading) {
      update(s => ({ ...s, loading }));
    },

    setError(error) {
      update(s => ({ ...s, error, loading: false }));
    },

    clearError() {
      update(s => ({ ...s, error: null }));
    },
  };
}

export const auth = createAuthStore();

export const isAuthenticated = derived(auth, $auth => !!$auth.token);

export const selectedNav = writable(localStorage.getItem(NAV_KEY) || '/');

selectedNav.subscribe(value => {
  localStorage.setItem(NAV_KEY, value);
});
