// Estúdio (B366): o build sai em `web-admin/backend/static/estudio/`, servido pelo FastAPI na mesma origem.
// A CSP do painel só permite `self` (`painel/aplicacao.py:70`): nada de script ou estilo embutido no HTML —
// o polyfill de modulepreload (script inline) fica desligado e o CSS sai em arquivo.
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  base: "/static/estudio/",
  build: {
    outDir: "../backend/static/estudio",
    emptyOutDir: true,
    modulePreload: { polyfill: false },
    assetsInlineLimit: 0,
    cssCodeSplit: false,
  },
  // Só no `npm run dev`: o painel recusa escrita de outra origem; o proxy fala com ele como a própria página dele.
  server: {
    proxy: {
      "/api": {
        target: "http://localhost:8000",
        changeOrigin: true,
        configure: (proxy) => proxy.on("proxyReq", (req) => {
          if (req.getHeader("origin")) req.setHeader("origin", "http://localhost:8000");
          if (req.getHeader("referer")) req.setHeader("referer", "http://localhost:8000/");
        }),
      },
    },
  },
  test: { include: ["src/**/*.test.ts"], environment: "node" },
});
