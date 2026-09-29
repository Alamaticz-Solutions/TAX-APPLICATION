import { copyFile, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const componentRoot = resolve(scriptDir, "..");
const pdsRoot = resolve(componentRoot, "..");
const distRoot = resolve(componentRoot, "dist");

const componentStyles = await readFile(resolve(componentRoot, "src/styles.css"), "utf8");
const packagedStyles = componentStyles.replace(
  '@import "../../tokens/pdsTokens.css";',
  '@import "./tokens.css";'
);

if (packagedStyles === componentStyles) {
  throw new Error("component stylesheet no longer has the canonical token import");
}

const canonicalTokens = await readFile(resolve(pdsRoot, "tokens/pdsTokens.css"), "utf8");
const reviewOnlyFontFaces = [
  `@font-face {
  font-family: "InterVariable";
  src: url("./fonts/InterVariable.woff2") format("woff2");
  font-style: normal;
  font-weight: 100 900;
  font-display: swap;
}

`,
  `@font-face {
  font-family: "Geist Mono Variable";
  src: url("./fonts/GeistMonoVariable.woff2") format("woff2");
  font-style: normal;
  font-weight: 100 900;
  font-display: swap;
}

`
];

let packagedTokens = canonicalTokens;
for (const fontFace of reviewOnlyFontFaces) {
  if (!packagedTokens.includes(fontFace)) {
    throw new Error("review-only webfont declaration changed without a packaging decision");
  }
  packagedTokens = packagedTokens.replace(fontFace, "");
}
const packagedFontReferences = [...packagedTokens.matchAll(/url\("\.\/fonts\/([^"]+)"\)/g)]
  .map((match) => match[1]);
if (
  packagedFontReferences.length !== 1
  || packagedFontReferences[0] !== "Poppins-Bold-latin.woff2"
) {
  throw new Error("packaged tokens must contain only the governed Poppins display font");
}

await mkdir(distRoot, { recursive: true });
await rm(resolve(distRoot, "fonts"), { recursive: true, force: true });
await mkdir(resolve(distRoot, "fonts"), { recursive: true });
await copyFile(
  resolve(pdsRoot, "tokens/fonts/Poppins-Bold-latin.woff2"),
  resolve(distRoot, "fonts/Poppins-Bold-latin.woff2")
);
await copyFile(
  resolve(pdsRoot, "tokens/fonts/Poppins-OFL.txt"),
  resolve(distRoot, "fonts/Poppins-OFL.txt")
);
await writeFile(resolve(distRoot, "styles.css"), packagedStyles);
await writeFile(resolve(distRoot, "tokens.css"), packagedTokens);
