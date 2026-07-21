import { writable, derived } from 'svelte/store';

const TOKEN_KEY = 'admin_token';
const USERNAME_KEY = 'admin_username';
const IS_ADMIN_KEY = 'admin_is_admin';
const NAV_KEY = 'admin_selected_nav';

function createAuthStore() {
  const { subscribe, set, update } = writable({
    token: localStorage.getItem(TOKEN_KEY) || null,
    username: localStorage.getItem(USERNAME_KEY) || null,
    is_admin: localStorage.getItem(IS_ADMIN_KEY) === 'true',
    loading: false,
    error: null,
  });

  return {
    subscribe,

    login(token, username, is_admin = false) {
      localStorage.setItem(TOKEN_KEY, token);
      localStorage.setItem(USERNAME_KEY, username);
      localStorage.setItem(IS_ADMIN_KEY, String(is_admin));
      set({ token, username, is_admin, loading: false, error: null });
    },

    logout() {
      localStorage.removeItem(TOKEN_KEY);
      localStorage.removeItem(USERNAME_KEY);
      localStorage.removeItem(IS_ADMIN_KEY);
      localStorage.removeItem(NAV_KEY);
      set({ token: null, username: null, is_admin: false, loading: false, error: null });
      // 延迟导入权限 store 避免循环依赖
      setTimeout(async () => {
        const { permissionStore } = await import('./permission.js');
        permissionStore.reset();
      }, 0);
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
export const isAdmin = derived(auth, $auth => $auth.is_admin);

export const selectedNav = writable(localStorage.getItem(NAV_KEY) || '/');

selectedNav.subscribe(value => {
  localStorage.setItem(NAV_KEY, value);
});
