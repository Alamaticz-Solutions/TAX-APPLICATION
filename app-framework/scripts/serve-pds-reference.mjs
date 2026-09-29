#!/usr/bin/env node
import fs from "node:fs";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const designSystemRoot = path.join(repoRoot, "appfw_ui/pds_health");
const host = process.env.PDS_REFERENCE_HOST ?? "127.0.0.1";
const port = Number(process.env.PDS_REFERENCE_PORT ?? "5175");

const mimeTypes = new Map([
  [".css", "text/css; charset=utf-8"],
  [".html", "text/html; charset=utf-8"],
  [".js", "text/javascript; charset=utf-8"],
  [".json", "application/json; charset=utf-8"],
  [".svg", "image/svg+xml"]
]);

function safePathFromUrl(url) {
  const parsed = new URL(url, `http://${host}:${port}`);
  const pathname = decodeURIComponent(parsed.pathname);
  let relativePath = pathname === "/" ? "reference/index.html" : pathname.replace(/^\/+/, "");
  if (relativePath.endsWith("/")) {
    relativePath = `${relativePath}index.html`;
  }
  const absolutePath = path.resolve(designSystemRoot, relativePath);
  if (!absolutePath.startsWith(`${designSystemRoot}${path.sep}`) && absolutePath !== designSystemRoot) {
    return null;
  }
  return absolutePath;
}

const server = http.createServer((request, response) => {
  const filePath = safePathFromUrl(request.url ?? "/");
  if (!filePath) {
    response.writeHead(403);
    response.end("Forbidden");
    return;
  }

  fs.readFile(filePath, (error, contents) => {
    if (error) {
      response.writeHead(error.code === "ENOENT" ? 404 : 500);
      response.end(error.code === "ENOENT" ? "Not found" : "Server error");
      return;
    }

    const contentType = mimeTypes.get(path.extname(filePath)) ?? "application/octet-stream";
    response.writeHead(200, {
      "cache-control": "no-cache",
      "content-type": contentType
    });
    response.end(contents);
  });
});

server.listen(port, host, () => {
  console.log(`PDS reference preview: http://${host}:${port}/reference/`);
});
