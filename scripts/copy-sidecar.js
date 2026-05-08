// Copy the built modutone-worker binary to src-tauri/binaries/ with
// the Tauri-required target-triple suffix so that `externalBin` can
// bundle it alongside the main executable at install time.
//
// Usage:
//   node scripts/copy-sidecar.js            (release profile)
//   node scripts/copy-sidecar.js --debug     (debug profile)

import { execSync } from "node:child_process";
import { copyFileSync, mkdirSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, "..");

// Parse --debug flag
const profile = process.argv.includes("--debug") ? "debug" : "release";

// Detect the Rust host triple
const rustcOutput = execSync("rustc -vV", { encoding: "utf-8" });
const hostLine = rustcOutput
  .split("\n")
  .find((line) => line.startsWith("host:"));
if (!hostLine) {
  console.error("ERROR: Could not determine Rust host triple from `rustc -vV`");
  process.exit(1);
}
const triple = hostLine.replace("host:", "").trim();

// Platform-specific extension
const ext = process.platform === "win32" ? ".exe" : "";

// Source: the built worker binary in target/{profile}/
const src = join(root, "target", profile, `modutone-worker${ext}`);

// Destination: src-tauri/binaries/modutone-worker-{triple}[.exe]
const destDir = join(root, "src-tauri", "binaries");
const dest = join(destDir, `modutone-worker-${triple}${ext}`);

if (!existsSync(src)) {
  console.error(`ERROR: Worker binary not found at ${src}`);
  console.error(
    profile === "release"
      ? "Build the worker first: cargo build --release -p modutone-worker"
      : "Build the worker first: cargo build -p modutone-worker",
  );
  process.exit(1);
}

mkdirSync(destDir, { recursive: true });
copyFileSync(src, dest);
console.log(`Copied sidecar (${profile}): ${src} -> ${dest}`);
