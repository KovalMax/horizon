import { defineConfig } from 'vite';

export default defineConfig({
  server: {
    allowedHosts: ['horizon.localhost'],
    hmr: { protocol: 'wss', host: 'horizon.localhost', clientPort: 443 },
    watch: { usePolling: true, interval: 300 },
  },
});
