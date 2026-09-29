import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";

const componentRoot = fileURLToPath(new URL("./", import.meta.url));
const entryNames = [
  "ambient",
  "catalog",
  "charts",
  "conversation",
  "conversation-workspace",
  "connected-fabric",
  "copy",
  "data",
  "data-grid",
  "experience",
  "exploration-workspace",
  "foundation",
  "forms",
  "intelligence-presentation",
  "intelligence-presentation-model",
  "ix-recipes",
  "layout",
  "narrative-workspace",
  "primitives",
  "process",
  "relationship-atlas",
  "surfaces",
  "timeline",
  "types",
  "work-surfaces"
];

const entries = Object.fromEntries([
  ["index", resolve(componentRoot, "src/index.ts")],
  ...entryNames.map((name) => [
    name,
    resolve(componentRoot, `src/${name}.${name === "catalog" || name === "copy" || name === "intelligence-presentation-model" || name === "types" ? "ts" : "tsx"}`)
  ])
]);

const externalPackages = new Set([
  "@appfw/pds-ix-presentation-contract",
  "react",
  "react-dom",
  "react-dom/client",
  "react-dom/server",
  "react/jsx-runtime",
  "react/jsx-dev-runtime",
  "react-aria-components",
  "ag-grid-community",
  "ag-grid-react",
  "@assistant-ui/react",
  "@xyflow/react"
]);

export default defineConfig({
  build: {
    target: "es2020",
    outDir: "dist",
    emptyOutDir: true,
    sourcemap: false,
    minify: false,
    lib: {
      entry: entries,
      formats: ["es"]
    },
    rollupOptions: {
      external(id) {
        return externalPackages.has(id)
          || id.startsWith("react/")
          || id.startsWith("@react-aria/")
          || id.startsWith("@internationalized/");
      },
      output: {
        entryFileNames: "[name].js",
        chunkFileNames: "chunks/[name]-[hash].js"
      }
    }
  },
  esbuild: {
    jsx: "automatic"
  }
});
