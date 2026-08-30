<script>
  import { onMount } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { api } from '../lib/api.js';
  import { addToast } from '../stores/app.js';
  import LoadingSpinner from '../components/LoadingSpinner.svelte';

  let user = null;
  let userOrgs = [];
  let loading = true;
  let error = '';
  let editing = false;
  let editForm = { display_name: '', email: '', avatar_url: '' };
  let saving = false;

  let showPasswordModal = false;
  let newPassword = '';
  let confirmPassword = '';
  let changingPassword = false;

  onMount(async () => {
    await loadUser();
  });

  async function loadUser() {
    loading = true;
    error = '';
    try {
      const [me, orgs] = await Promise.all([
        api.getMe(),
        api.getUserOrgs().catch(() => []),
      ]);
      user = me;
      // /users/me/orgs 已支持 tenant_admin 视角，会包含用户直接加入的组织 + 租户下管理的组织
      userOrgs = orgs || [];
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  function startEdit() {
    editForm = {
      display_name: user.display_name || '',
      email: user.email || '',
      avatar_url: user.avatar_url || ''
    };
    editing = true;
  }

  async function handleSave() {
    saving = true;
    try {
      await api.updateMe(editForm);
      user = { ...user, ...editForm };
      editing = false;
      addToast('Profile updated', 'success');
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      saving = false;
    }
  }

  async function handleChangePassword() {
    if (!newPassword || !confirmPassword) {
      addToast('Please fill in both fields', 'error');
      return;
    }
    if (newPassword !== confirmPassword) {
      addToast('Passwords do not match', 'error');
      return;
    }
    if (newPassword.length < 6) {
      addToast('Password must be at least 6 characters', 'error');
      return;
    }
    changingPassword = true;
    try {
      await api.updateMe({ password: newPassword });
      newPassword = '';
      confirmPassword = '';
      showPasswordModal = false;
      addToast('Password changed successfully', 'success');
    } catch (e) {
      addToast(e.message, 'error');
    } finally {
      changingPassword = false;
    }
  }

  function getRoleColor(role) {
    switch (role) {
      case 'owner': return 'bg-purple-100 text-purple-700';
      case 'admin': return 'bg-blue-100 text-blue-700';
      case 'developer': return 'bg-emerald-100 text-emerald-700';
      case 'reviewer': return 'bg-amber-100 text-amber-700';
      case 'member': return 'bg-gray-100 text-gray-700';
      default: return 'bg-gray-100 text-gray-700';
    }
  }

  function getTypeColor(type) {
    switch (type) {
      case 'user': return 'bg-blue-100 text-blue-700';
      case 'agent': return 'bg-purple-100 text-purple-700';
      case 'system': return 'bg-amber-100 text-amber-700';
      default: return 'bg-gray-100 text-gray-700';
    }
  }

  function getAvatarInitials(name) {
    if (!name) return '?';
    const parts = name.trim().split(' ');
    if (parts.length >= 2) {
      return (parts[0][0] + parts[1][0]).toUpperCase();
    }
    return name[0].toUpperCase();
  }

  function getAvatarColor(id) {
    if (!id) return 'from-indigo-500 to-purple-600';
    const colors = [
      'from-indigo-500 to-purple-600',
      'from-emerald-500 to-teal-600',
      'from-orange-500 to-red-600',
      'from-violet-500 to-indigo-600',
      'from-pink-500 to-rose-600',
      'from-cyan-500 to-blue-600'
    ];
    const idx = id.charCodeAt(0) % colors.length;
    return colors[idx];
  }
</script>

<div class="p-8">
  <div class="page-header">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-[28px] font-extrabold text-gray-800 tracking-tight">My Profile</h1>
        <p class="text-gray-500 text-sm mt-1.5 font-medium">Manage your account information</p>
      </div>
      {#if !editing}
        <button
          on:click={startEdit}
          class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm flex items-center gap-2"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z"/></svg>
          Edit Profile
        </button>
      {/if}
    </div>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <div class="bg-rose-50 border border-rose-100 text-rose-600 px-5 py-4 rounded-2xl text-sm font-medium">{error}</div>
  {:else if user}
    <div class="grid grid-cols-1 xl:grid-cols-3 gap-6">
      <div class="xl:col-span-2 space-y-6">
        <div class="bg-white rounded-2xl border border-gray-200 shadow-card">
          <div class="px-6 py-5 border-b border-gray-100">
            <h2 class="font-semibold text-gray-800 text-sm">Account Information</h2>
          </div>
          <div class="p-6">
            {#if editing}
              <div class="space-y-4">
                <div>
                  <label for="display-name" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Display Name</label>
                  <input
                    id="display-name"
                    type="text"
                    bind:value={editForm.display_name}
                    placeholder="Your display name"
                    class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white"
                  />
                </div>
                <div>
                  <label for="email" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Email</label>
                  <input
                    id="email"
                    type="email"
                    bind:value={editForm.email}
                    placeholder="email@example.com"
                    class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white"
                  />
                </div>
                <div>
                  <label for="avatar-url" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Avatar URL</label>
                  <input
                    id="avatar-url"
                    type="url"
                    bind:value={editForm.avatar_url}
                    placeholder="https://example.com/avatar.png"
                    class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white"
                  />
                </div>
                <div class="flex gap-3 justify-end pt-2">
                  <button
                    on:click={() => editing = false}
                    class="px-4 py-2.5 text-gray-500 hover:text-gray-800 font-semibold text-sm transition-all rounded-lg hover:bg-gray-50"
                  >
                    Cancel
                  </button>
                  <button
                    on:click={handleSave}
                    disabled={saving}
                    class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    {saving ? 'Saving...' : 'Save Changes'}
                  </button>
                </div>
              </div>
            {:else}
              <div class="space-y-5">
                <div class="flex items-start gap-5">
                  {#if user.avatar_url}
                    <img
                      src={user.avatar_url}
                      alt={user.username}
                      class="w-16 h-16 rounded-2xl object-cover ring-2 ring-indigo-100"
                    />
                  {:else}
                    <div class="w-16 h-16 rounded-2xl bg-gradient-to-br {getAvatarColor(user.id)} flex items-center justify-center text-white text-xl font-bold shadow-glow ring-2 ring-indigo-100">
                      {getAvatarInitials(user.display_name || user.username)}
                    </div>
                  {/if}
                  <div class="flex-1">
                    <div class="flex items-center gap-3 mb-1">
                      <h3 class="text-gray-800 font-bold text-lg">{user.display_name || user.username}</h3>
                      <span class="px-2 py-0.5 rounded-full text-xs font-medium {getTypeColor(user.identity_type)}">
                        {user.identity_type}
                      </span>
                    </div>
                    <p class="text-gray-400 text-sm font-mono">@{user.username}</p>
                    {#if user.email}
                      <p class="text-gray-400 text-sm mt-1">{user.email}</p>
                    {/if}
                  </div>
                </div>
                <div class="grid grid-cols-2 gap-4 pt-4 border-t border-gray-100">
                  <div>
                    <p class="text-gray-400 text-xs font-semibold uppercase tracking-wider mb-1">User ID</p>
                    <p class="text-gray-600 text-sm font-mono">{user.id}</p>
                  </div>
                  <div>
                    <p class="text-gray-400 text-xs font-semibold uppercase tracking-wider mb-1">Created</p>
                    <p class="text-gray-600 text-sm">{new Date(user.created_at).toLocaleDateString()}</p>
                  </div>
                </div>
              </div>
            {/if}
          </div>
        </div>

        <div class="bg-white rounded-2xl border border-gray-200 shadow-card">
          <div class="px-6 py-5 border-b border-gray-200 flex items-center justify-between">
            <h2 class="font-semibold text-gray-800 text-sm">Security</h2>
          </div>
          <div class="p-6">
            <div class="flex items-center justify-between">
              <div>
                <p class="text-gray-800 font-semibold text-sm">Password</p>
                <p class="text-gray-400 text-xs mt-0.5">Last changed: never</p>
              </div>
              <button
                on:click={() => { showPasswordModal = true; newPassword = ''; confirmPassword = ''; }}
                class="btn-secondary px-4 py-2 rounded-xl font-semibold text-sm"
              >
                Change Password
              </button>
            </div>
          </div>
        </div>
      </div>

      {#if userOrgs.length > 0}
      <div class="space-y-6">
        <div class="bg-white rounded-2xl border border-gray-200 shadow-card">
          <div class="px-6 py-5 border-b border-emerald-100/60">
            <h2 class="font-semibold text-gray-800 text-sm">My Organizations ({userOrgs.length})</h2>
          </div>
          <div class="p-4">
            <div class="space-y-3">
              {#each userOrgs as org (org.id)}
                <a
                  href={"/organizations/" + org.id}
                  class="flex items-center gap-3 p-3 rounded-xl hover:bg-gray-50 transition-all group"
                >
                  <div class="w-9 h-9 rounded-xl bg-blue-600 flex items-center justify-center text-white text-xs font-bold shadow-glow flex-shrink-0 group-hover:scale-105 transition-transform">
                    {org.name[0]?.toUpperCase() || '?'}
                  </div>
                  <div class="flex-1 min-w-0">
                    <p class="text-gray-800 font-semibold text-sm truncate">{org.name}</p>
                    <p class="text-gray-400 text-xs truncate">{org.slug || '—'}</p>
                  </div>
                  <span class="px-2 py-1 rounded-full text-xs font-medium {getRoleColor(org.role)}">
                    {org.role}
                  </span>
                </a>
              {/each}
            </div>
          </div>
        </div>
      </div>
      {/if}
    </div>
  {/if}
</div>

{#if showPasswordModal}
<div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50 modal-overlay">
  <div class="bg-white rounded-2xl p-6 w-full max-w-md shadow-elevated-lg border border-gray-200 modal-content">
    <h2 class="text-lg font-bold text-gray-800 mb-5">Change Password</h2>
    <div class="space-y-4">
      <div>
        <label for="new-password" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">New Password</label>
        <input
          id="new-password"
          type="password"
          bind:value={newPassword}
          placeholder="At least 6 characters"
          class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white"
        />
      </div>
      <div>
        <label for="confirm-password" class="block text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Confirm Password</label>
        <input
          id="confirm-password"
          type="password"
          bind:value={confirmPassword}
          placeholder="Re-enter password"
          class="w-full px-4 py-3 border border-gray-200 rounded-xl text-sm input-focus outline-none font-medium bg-white"
        />
      </div>
      <div class="flex gap-3 justify-end pt-1">
        <button
          on:click={() => { showPasswordModal = false; newPassword = ''; confirmPassword = ''; }}
          class="px-4 py-2.5 text-gray-500 hover:text-gray-800 font-semibold text-sm transition-all rounded-lg hover:bg-gray-50"
        >
          Cancel
        </button>
        <button
          on:click={handleChangePassword}
          disabled={changingPassword || !newPassword || !confirmPassword}
          class="btn-primary px-5 py-2.5 rounded-xl font-semibold text-sm disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {changingPassword ? 'Changing...' : 'Change Password'}
        </button>
      </div>
    </div>
  </div>
</div>
{/if}
