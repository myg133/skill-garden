/**
 * svelte-i18n 初始化配置
 * 支持浏览器语言自动检测、localStorage 持久化和 URL 参数覆盖
 */

import { register, init, getLocaleFromNavigator, locale } from 'svelte-i18n';

// 注册语言包
register('en', () => import('./en.json'));
register('zh', () => import('./zh.json'));

// localStorage key
const LOCALE_STORAGE_KEY = 'aionhive_locale';

/**
 * 获取初始语言
 * 优先级：URL参数 > localStorage > 浏览器语言 > 默认英文
 */
function getInitialLocale() {
  // 1. URL 参数检查 (?lang=zh 或 ?lang=en)
  if (typeof window !== 'undefined') {
    const urlParams = new URLSearchParams(window.location.search);
    const langParam = urlParams.get('lang');
    if (langParam && ['en', 'zh'].includes(langParam.toLowerCase())) {
      return langParam.toLowerCase();
    }
  }

  // 2. localStorage 检查
  if (typeof localStorage !== 'undefined') {
    const savedLocale = localStorage.getItem(LOCALE_STORAGE_KEY);
    if (savedLocale && ['en', 'zh'].includes(savedLocale)) {
      return savedLocale;
    }
  }

  // 3. 浏览器语言检测
  if (typeof navigator !== 'undefined') {
    const browserLocale = navigator.language || navigator.userLanguage || '';
    if (browserLocale.startsWith('zh')) {
      return 'zh';
    }
    if (browserLocale.startsWith('en')) {
      return 'en';
    }
  }

  // 4. 默认英文
  return 'en';
}

// 初始化 i18n
init({
  fallbackLocale: 'en',
  initialLocale: getInitialLocale(),
});

// 保存语言偏好到 localStorage
export function setLocale(newLocale) {
  if (newLocale && ['en', 'zh'].includes(newLocale)) {
    locale.set(newLocale);
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(LOCALE_STORAGE_KEY, newLocale);
    }
  }
}

// 获取当前语言
export function getLocale() {
  let current = 'en';
  locale.subscribe(value => {
    current = value || 'en';
  })();
  return current;
}

// 监听语言变化并保存
locale.subscribe(value => {
  if (value && typeof localStorage !== 'undefined') {
    localStorage.setItem(LOCALE_STORAGE_KEY, value);
  }
});

export { locale };
