import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

// In dev, proxy the generated GraphQL routes to the running CRM backend so the
// SPA can call them same-origin. Override the target with VITE_BACKEND_URL.
const backend = process.env.VITE_BACKEND_URL ?? "http://127.0.0.1:8080";
const packageJson = JSON.parse(readFileSync(new URL("./package.json", import.meta.url), "utf8")) as {
  version?: string;
};
const repoRoot = fileURLToPath(new URL("../../../../", import.meta.url));
const pdsTokensRoot = fileURLToPath(
  new URL("../../../../appfw_ui/pds_health/tokens/", import.meta.url)
);

export default defineConfig({
  plugins: [react(), tailwindcss()],
  // @appfw/pds-health-components resolves from the installed archive through package exports.
  resolve: {
    alias: {
      "@appfw/pds-health/tokens/pdsTokens.css": `${pdsTokensRoot}pdsTokens.css`,
      "@appfw/pds-health/tokens/pdsTokens.ts": `${pdsTokensRoot}pdsTokens.ts`
    },
    dedupe: ["react", "react-dom"]
  },
  define: {
    __APP_VERSION__: JSON.stringify(packageJson.version ?? "0.0.0")
  },
  server: {
    fs: {
      allow: [repoRoot]
    },
    port: 5173,
    proxy: {
      "/crm": backend,
      "/system": backend,
      "/admin/model": backend
    }
  },
  build: {
    outDir: "../backend/product_dist",
    emptyOutDir: true
  }
});
