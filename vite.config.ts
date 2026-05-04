import { defineConfig, loadEnv } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '')
  const target = env.VITE_MCP_URL || 'http://localhost:3000'

  return {
    plugins: [react(), tailwindcss()],
    server: {
      port: 5174,
      proxy: {
        '/api': { target, changeOrigin: true },
        '/health': { target, changeOrigin: true },
        '/mcp': { target, changeOrigin: true },
      },
    },
  }
})
