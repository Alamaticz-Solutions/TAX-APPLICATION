#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const frontendRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const commands = [
  ["appfw:check", ["npm", "run", "appfw:check", "--", "--json"]],
  ["test", ["npm", "test"]],
  ["build", ["npm", "run", "build"]]
];

for (const [label, [command, ...args]] of commands) {
  console.log(`\n[release:evidence] ${label}`);
  const result = spawnSync(command, args, {
    cwd: frontendRoot,
    stdio: "inherit"
  });

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

console.log("\n[release:evidence] ok");
