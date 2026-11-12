import { fileURLToPath, URL } from 'node:url'
import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

const BACKEND = process.env.TALKIE_BACKEND ?? 'http://127.0.0.1:3000'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
  },
  server: {
    // Same-origin in dev: the backend has no CORS or origin validation.
    proxy: {
      '/ws': { target: BACKEND, ws: true },
      '/health': { target: BACKEND },
    },
  },
})
