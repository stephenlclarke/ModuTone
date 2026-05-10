// Validate and report bundled model files for distribution.
//
// GGUF model files are too large for the NSIS archive (2 GB limit).
// They are distributed alongside the setup.exe and copied at install
// time by the NSIS POSTINSTALL hook. This script validates that the
// model files in src-tauri/resources/models/ match the catalog.
//
// Usage:
//   node scripts/prepare-models.js
//
// Reads from: src-tauri/resources/models/*.gguf + model_catalog.json

import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, "..");
const modelsDir = join(root, "src-tauri", "resources", "models");
const catalogPath = join(modelsDir, "model_catalog.json");

// Must match INSTALLED_SIZE_THRESHOLD in model_catalog.rs
const INSTALLED_SIZE_THRESHOLD = 0.9;

// Read catalog
const catalog = JSON.parse(readFileSync(catalogPath, "utf-8"));

// Find GGUF files in models directory (top-level only)
const ggufFiles = readdirSync(modelsDir)
  .filter((f) => f.toLowerCase().endsWith(".gguf"))
  .map((f) => ({
    filename: f,
    path: join(modelsDir, f),
    size: statSync(join(modelsDir, f)).size,
  }));

console.log(`Found ${ggufFiles.length} GGUF file(s) in ${modelsDir}`);
console.log();

let validCount = 0;
let skippedCount = 0;

for (const file of ggufFiles) {
  const entry = catalog.find((e) => e.filename === file.filename);
  if (!entry) {
    console.log(
      `  [UNCATALOGED] ${file.filename} (${(file.size / 1e9).toFixed(2)} GB)`,
    );
    validCount++;
    continue;
  }

  const minSize = Math.floor(entry.sizeBytes * INSTALLED_SIZE_THRESHOLD);
  if (file.size < minSize) {
    console.warn(
      `  [TRUNCATED]   ${file.filename} — ` +
        `${(file.size / 1e9).toFixed(2)} GB of expected ${(entry.sizeBytes / 1e9).toFixed(2)} GB ` +
        `(${((file.size / entry.sizeBytes) * 100).toFixed(1)}%)`,
    );
    skippedCount++;
  } else {
    console.log(
      `  [OK]          ${file.filename} (${(file.size / 1e9).toFixed(2)} GB)`,
    );
    validCount++;
  }
}

console.log();
console.log(`Models: ${validCount} valid, ${skippedCount} truncated/invalid`);

if (skippedCount > 0) {
  console.warn(
    "\nWarning: truncated models will not be discovered by the app.",
  );
  console.warn(
    "Re-download the full model files to include them in the distribution.",
  );
}

if (validCount === 0) {
  console.error(
    "\nERROR: No valid GGUF model files found. Packaging requires at least one valid model file.",
  );
  process.exit(1);
}
