import { writable, derived } from 'svelte/store';

const STORAGE_KEY = 'selected_org';

function loadSelected() {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    return saved ? JSON.parse(saved) : null;
  } catch {
    return null;
  }
}

function saveSelected(value) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(value));
  } catch {}
}

export const selectedOrg = writable(loadSelected());

selectedOrg.subscribe(value => {
  saveSelected(value);
});

// orgs list: [{ id, name, slug, role }] from /users/me orgs field
export const userOrgs = writable([]);

export const selectedOrgId = derived(selectedOrg, $o => $o?.id || null);
export const selectedOrgSlug = derived(selectedOrg, $o => $o?.slug || null);
export const selectedOrgRole = derived(selectedOrg, $o => $o?.role || null);
export const isPersonalSpace = derived(selectedOrg, $o => $o === null || $o?.id === '__personal__');
