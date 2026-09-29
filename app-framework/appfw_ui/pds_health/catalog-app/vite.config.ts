import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const appRoot = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(appRoot, "../../..");
const pdsComponentsRoot = path.resolve(appRoot, "../components/src");
const pdsTokensRoot = path.resolve(appRoot, "../tokens");

// The catalog app renders the real framework-owned components straight from
// source. Aliasing to source keeps the catalog honest: there is no separate
// build of the component package that could drift from what products import.
export default defineConfig({
  base: "./",
  plugins: [react()],
  resolve: {
    alias: {
      "react/jsx-runtime": path.resolve(appRoot, "node_modules/react/jsx-runtime.js"),
      "react-dom": path.resolve(appRoot, "node_modules/react-dom"),
      react: path.resolve(appRoot, "node_modules/react"),
      "@appfw/pds-health/tokens/pdsTokens.css": path.resolve(pdsTokensRoot, "pdsTokens.css"),
      "@appfw/pds-health-components/styles.css": path.resolve(pdsComponentsRoot, "styles.css"),
      "@appfw/pds-health-components/intelligence-presentation": path.resolve(pdsComponentsRoot, "intelligence-presentation.tsx"),
      "@appfw/pds-health-components/intelligence-presentation-model": path.resolve(pdsComponentsRoot, "intelligence-presentation-model.ts"),
      "@appfw/pds-health-components/ix-recipes": path.resolve(pdsComponentsRoot, "ix-recipes.tsx"),
      "@appfw/pds-ix-presentation-contract": path.resolve(appRoot, "../ix-presentation-contract/src/index.js"),
      "@appfw/pds-health-components": path.resolve(pdsComponentsRoot, "index.ts")
    }
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    rollupOptions: {
      input: {
        index: path.resolve(appRoot, "index.html"),
        "ix-reference": path.resolve(appRoot, "ix-reference.html"),
        "representative-interactions": path.resolve(appRoot, "f1-representative-interactions.html"),
        "neutral-work": path.resolve(appRoot, "pds-n0-neutral-my-work.html"),
        "trusted-task": path.resolve(appRoot, "pds-n1-trusted-task.html")
      }
    }
  },
  server: {
    port: 5176,
    fs: {
      allow: [repoRoot]
    }
  }
});
