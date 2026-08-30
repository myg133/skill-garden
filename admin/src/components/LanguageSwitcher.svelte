<script>
  import { locale, _ } from 'svelte-i18n';
  import { setLocale } from '../i18n/index.js';

  let isOpen = false;

  function toggleDropdown() {
    isOpen = !isOpen;
  }

  function closeDropdown() {
    isOpen = false;
  }

  function switchLanguage(lang) {
    setLocale(lang);
    isOpen = false;
  }

  $: currentLocale = $locale || 'en';
  $: currentLangLabel = currentLocale === 'zh' ? '中文' : 'EN';
  $: otherLangLabel = currentLocale === 'zh' ? 'EN' : '中文';
  $: otherLang = currentLocale === 'zh' ? 'en' : 'zh';
</script>

<svelte:window on:click={closeDropdown} />

<div class="relative">
  <button
    on:click|stopPropagation={toggleDropdown}
    class="flex items-center gap-1.5 px-3 py-2 rounded-lg text-sm font-medium transition-all duration-200
           {isOpen ? 'bg-blue-50 text-blue-600' : 'text-gray-600 hover:bg-gray-100 hover:text-gray-800'}"
    aria-label="Switch language"
  >
    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129"/>
    </svg>
    <span>{currentLangLabel}</span>
    <svg class="w-3 h-3 transition-transform duration-200 {isOpen ? 'rotate-180' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/>
    </svg>
  </button>

  {#if isOpen}
    <div
      class="absolute right-0 mt-2 w-32 bg-white rounded-lg shadow-lg border border-gray-200 py-1 z-50"
      on:click|stopPropagation
    >
      <button
        on:click={() => switchLanguage('en')}
        class="w-full px-4 py-2.5 text-left text-sm hover:bg-gray-50 transition-colors flex items-center gap-2
               {currentLocale === 'en' ? 'text-blue-600 font-medium' : 'text-gray-700'}"
      >
        <span class="text-base">🇺🇸</span>
        <span>English</span>
        {#if currentLocale === 'en'}
          <svg class="w-4 h-4 ml-auto text-blue-500" fill="currentColor" viewBox="0 0 20 20">
            <path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd"/>
          </svg>
        {/if}
      </button>
      <button
        on:click={() => switchLanguage('zh')}
        class="w-full px-4 py-2.5 text-left text-sm hover:bg-gray-50 transition-colors flex items-center gap-2
               {currentLocale === 'zh' ? 'text-blue-600 font-medium' : 'text-gray-700'}"
      >
        <span class="text-base">🇨🇳</span>
        <span>中文</span>
        {#if currentLocale === 'zh'}
          <svg class="w-4 h-4 ml-auto text-blue-500" fill="currentColor" viewBox="0 0 20 20">
            <path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd"/>
          </svg>
        {/if}
      </button>
    </div>
  {/if}
</div>
