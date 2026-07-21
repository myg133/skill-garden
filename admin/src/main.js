import App from './App.svelte';
import './app.css';

const app = new App({
  target: document.getElementById('app'),
  props: {
    url: window.location.pathname + window.location.search
  }
});

// 支持浏览器前进/后退按钮
window.addEventListener('popstate', () => {
  app.$set({ url: window.location.pathname + window.location.search });
});

export default app;
