#!/usr/bin/env node
import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { copyFile, mkdir, mkdtemp, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, "../..");
const componentRoot = resolve(repoRoot, "appfw_ui/pds_health/components");
const packageDocument = JSON.parse(await readFile(resolve(componentRoot, "package.json"), "utf8"));
const archiveName = `appfw-pds-health-components-${packageDocument.version}.tgz`;
const archivePath = resolve(repoRoot, "target/appfw/packages", archiveName);
const proofRoot = await mkdtemp(resolve(tmpdir(), "appfw-pds-package-consumer-"));
const npmCache = resolve(repoRoot, "target/appfw/npm-cache");
const evidencePath = resolve(repoRoot, "target/appfw/pds-package-consumer-evidence.json");

try {
  await mkdir(resolve(proofRoot, "src"), { recursive: true });
  await mkdir(npmCache, { recursive: true });
  await copyFile(archivePath, resolve(proofRoot, archiveName));
  const archiveReference = `file:./${archiveName}`;

  await writeFile(resolve(proofRoot, "package.json"), `${JSON.stringify({
    name: "pds-package-only-consumer-proof",
    version: "1.0.0",
    private: true,
    type: "module",
    scripts: { build: "tsc --noEmit && vite build" },
    dependencies: {
      "@appfw/pds-health-components": archiveReference,
      react: "^18.3.1",
      "react-dom": "^18.3.1"
    },
    devDependencies: {
      "@types/react": "^18.3.12",
      "@types/react-dom": "^18.3.1",
      typescript: "^5.6.3",
      vite: "^6.4.2"
    }
  }, null, 2)}\n`);
  await writeFile(resolve(proofRoot, "tsconfig.json"), `${JSON.stringify({
    compilerOptions: {
      target: "ES2020",
      lib: ["DOM", "DOM.Iterable", "ES2020"],
      strict: true,
      skipLibCheck: true,
      module: "ESNext",
      moduleResolution: "Bundler",
      jsx: "react-jsx",
      noEmit: true
    },
    include: ["src"]
  }, null, 2)}\n`);
  await writeFile(
    resolve(proofRoot, "index.html"),
    '<div id="root"></div><script type="module" src="/src/main.tsx"></script>\n'
  );
  await writeFile(resolve(proofRoot, "src/main.tsx"), `
import React from "react";
import { createRoot } from "react-dom/client";
import { Badge, Button, SegmentedControl } from "@appfw/pds-health-components/primitives";
import { AppShell, PageHeader } from "@appfw/pds-health-components/layout";
import { Surface } from "@appfw/pds-health-components/surfaces";
import "@appfw/pds-health-components/styles.css";

function App() {
  return (
    <AppShell
      brand={<strong>Independent product</strong>}
      navigation={<Button variant="quiet">My work</Button>}
    >
        <main>
          <PageHeader title="Package-only consumer" />
          <Surface>
            <SegmentedControl
              ariaLabel="Work status"
              value="open"
              options={[{ label: "Open", value: "open" }, { label: "Done", value: "done" }]}
              onValueChange={() => undefined}
            />
            <Badge>3 items</Badge>
          </Surface>
        </main>
    </AppShell>
  );
}

createRoot(document.getElementById("root")!).render(<App />);
`);

  function run(command, args) {
    const result = spawnSync(command, args, {
      cwd: proofRoot,
      encoding: "utf8",
      env: { ...process.env, npm_config_cache: npmCache }
    });
    if (result.status !== 0) {
      throw new Error(`${command} ${args.join(" ")} failed\n${result.stdout}\n${result.stderr}`);
    }
    return result.stdout.trim();
  }

  const installOutput = run("npm", [
    "install",
    "--ignore-scripts",
    "--no-audit",
    "--no-fund",
    "--prefer-offline"
  ]);
  const buildOutput = run("npm", ["run", "build"]);

  async function filesBelow(directory) {
    const entries = await readdir(directory, { withFileTypes: true });
    const nested = await Promise.all(entries.map(async (entry) => {
      const path = resolve(directory, entry.name);
      return entry.isDirectory() ? filesBelow(path) : [path];
    }));
    return nested.flat();
  }

  const distFiles = await filesBelow(resolve(proofRoot, "dist"));
  const escapedSourcePaths = [];
  for (const file of distFiles.filter((candidate) => /\.(js|css|html)$/.test(candidate))) {
    const text = await readFile(file, "utf8");
    if (
      text.includes("appfw_ui/pds_health")
      || text.includes(repoRoot)
      || text.includes(".tsx")
    ) {
      escapedSourcePaths.push(relative(proofRoot, file));
    }
  }
  if (escapedSourcePaths.length > 0) {
    throw new Error(`framework source escaped into consumer: ${escapedSourcePaths.join(", ")}`);
  }

  const archive = await readFile(archivePath);
  const lock = JSON.parse(await readFile(resolve(proofRoot, "package-lock.json"), "utf8"));
  const packageLockEntry = lock.packages?.["node_modules/@appfw/pds-health-components"];
  if (!packageLockEntry?.integrity) {
    throw new Error("consumer lock is missing PDS archive integrity");
  }

  const evidence = {
    schema: "appfw_pds_package_consumer_evidence@1",
    ok: true,
    generated_at: new Date().toISOString(),
    package: {
      name: packageDocument.name,
      version: packageDocument.version,
      archive: relative(repoRoot, archivePath),
      sha256: createHash("sha256").update(archive).digest("hex"),
      lock_integrity: packageLockEntry.integrity
    },
    consumer: {
      root_kind: "detached-temporary-directory",
      dependency: archiveReference,
      framework_checkout_aliases: false,
      family_subpaths_proven: ["primitives", "layout", "surfaces"],
      declaration_and_runtime_build_proven: true,
      dist_files: distFiles.map((file) => relative(proofRoot, file)).sort()
    },
    commands: {
      install: installOutput.split("\n").slice(-3),
      build: buildOutput.split("\n").slice(-8)
    }
  };
  await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
  process.stdout.write(`${JSON.stringify(evidence, null, 2)}\n`);
} finally {
  await rm(proofRoot, { recursive: true, force: true });
}
