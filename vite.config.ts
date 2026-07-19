import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import path from 'path';

// https://vite.dev/config/
export default defineConfig({
  plugins: [svelte()],
  // Node 17+ 預設 verbatim DNS：localhost 會優先解析成 IPv6 (::1)，
  // 使 Vite 只綁 [::1]，而 Tauri WebView 走 IPv4 (127.0.0.1) 連不到 → 空白頁。
  // 明確綁 127.0.0.1，與 tauri.conf.json 的 devUrl 一致。
  server: {
    host: '127.0.0.1',
    port: 5173,
    strictPort: true,
    // Cargo 會在開發模式持續重建並鎖定 DLL；Vite 不需要監看 Rust 建置輸出。
    // Windows 上若嘗試監看被鎖定的 DLL，Node.js 會以 EBUSY 終止整個 dev server。
    watch: {
      ignored: ['**/src-tauri/target/**'],
    },
  },
  resolve: {
    conditions: ['browser'],
    alias: {
      $lib: path.resolve('./src/lib'),
    },
  },
  test: {
    environment: 'jsdom',
    globals: true,
    include: ['src/**/*.test.ts'],
  },
});
