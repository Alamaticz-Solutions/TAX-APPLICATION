import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const pdsComponentsRoot = path.resolve(repoRoot, "appfw_ui/pds_health/components/src");
const pdsTokensRoot = path.resolve(repoRoot, "appfw_ui/pds_health/tokens");

export default defineConfig({
  base: "/admin/",
  plugins: [react()],
  define: {
    __ADMIN_UI_VERSION__: JSON.stringify(process.env.npm_package_version ?? "0.0.0")
  },
  resolve: {
    alias: {
      "react/jsx-runtime": path.resolve(repoRoot, "admin_ui/node_modules/react/jsx-runtime.js"),
      "react-dom": path.resolve(repoRoot, "admin_ui/node_modules/react-dom"),
      react: path.resolve(repoRoot, "admin_ui/node_modules/react"),
      "@appfw/pds-health/tokens/pdsTokens.css": path.resolve(pdsTokensRoot, "pdsTokens.css"),
      "@appfw/pds-health/tokens/pdsTokens.ts": path.resolve(pdsTokensRoot, "pdsTokens.ts"),
      "@appfw/pds-health-components/styles.css": path.resolve(pdsComponentsRoot, "styles.css"),
      "@appfw/pds-health-components": path.resolve(pdsComponentsRoot, "index.ts")
    }
  },
  build: {
    outDir: "../backend/admin_dist",
    emptyOutDir: true,
    rollupOptions: {
      output: {
        // Keep a single entry chunk so /admin/assets is easy to serve, but do NOT
        // inline into index.html — managed CSP is `script-src 'self'` / `style-src 'self'`
        // (ENV_NAME=dev|tst|stg|prd) and blocks unsafe-inline.
        inlineDynamicImports: true
      }
    }
  },
  server: {
    port: 5173,
    fs: {
      allow: [repoRoot]
    },
    proxy: {
      "/admin/model": "http://127.0.0.1:8080",
      "/crm": "http://127.0.0.1:8080",
      "/mft": "http://127.0.0.1:8080",
      "/system": "http://127.0.0.1:8080"
    }
  }
});
