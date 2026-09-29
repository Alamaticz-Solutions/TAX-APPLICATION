import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";

const root = fileURLToPath(new URL("./", import.meta.url));

export default defineConfig({
  build: {
    target: "es2020",
    outDir: "dist",
    emptyOutDir: true,
    minify: false,
    sourcemap: false,
    lib: {
      entry: {
        index: resolve(root, "src/index.tsx"),
        "design-data": resolve(root, "src/design-data.ts"),
        "ix-recipe-projection": resolve(root, "src/ix-recipe-projection.ts"),
        "ix-recipes": resolve(root, "src/ix-recipes.tsx")
      },
      formats: ["es"]
    },
    rollupOptions: {
      external: ["@appfw/pds-ix-presentation-contract", "react", "react/jsx-runtime", "react-native"],
      output: { entryFileNames: "[name].js" }
    }
  }
});
