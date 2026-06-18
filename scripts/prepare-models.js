// Validate and report bundled model files for distribution.
//
// GGUF model files are too large for the NSIS archive (2 GB limit).
// They are distributed alongside the setup.exe and copied at install
// time by the NSIS POSTINSTALL hook. This script validates that the
// model files in src-tauri/resources/models/ match the catalog.
//
// Apple Silicon builds can also use MLX model directories. These are
// validated only on macOS arm64 because the runtime is platform specific.
//
// Usage:
//   node scripts/prepare-models.js
//
// Reads from: src-tauri/resources/models/* + model_catalog.json

import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, "..");
const modelsDir = join(root, "src-tauri", "resources", "models");
const catalogPath = join(modelsDir, "model_catalog.json");

// Must match INSTALLED_SIZE_THRESHOLD in model_catalog.rs
const INSTALLED_SIZE_THRESHOLD = 0.9;
const supportsMlx = process.platform === "darwin" && process.arch === "arm64";

function entryBackend(entry) {
  return entry.backend ?? "gguf";
}

function entryStoragePath(entry) {
  if (entryBackend(entry) === "mlx") {
    return entry.path ?? entry.filename;
  }
  return entry.filename ?? entry.path;
}

function entryStoragePaths(entry) {
  if (Array.isArray(entry.files) && entry.files.length > 0) {
    return entry.files;
  }
  const storagePath = entryStoragePath(entry);
  return storagePath ? [storagePath] : [];
}

function hasFileWithExtension(directory, extension) {
  return readdirSync(directory).some((file) =>
    file.toLowerCase().endsWith(extension),
  );
}

function looksLikeMlxModelDirectory(directory) {
  return (
    existsSync(join(directory, "config.json")) &&
    existsSync(join(directory, "tokenizer.json")) &&
    hasFileWithExtension(directory, ".safetensors")
  );
}

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

let validGgufCount = 0;
let validMlxCount = 0;
let skippedCount = 0;
const catalogedGgufFiles = new Set();

for (const entry of catalog.filter((e) => entryBackend(e) === "gguf")) {
  const files = entryStoragePaths(entry);
  for (const filename of files) {
    catalogedGgufFiles.add(filename);
  }

  const installedFiles = files
    .map((filename) => ({
      filename,
      path: join(modelsDir, filename),
    }))
    .filter((file) => existsSync(file.path) && statSync(file.path).isFile());

  const actualSize = installedFiles.reduce(
    (total, file) => total + statSync(file.path).size,
    0,
  );

  const minSize = Math.floor(entry.sizeBytes * INSTALLED_SIZE_THRESHOLD);
  if (installedFiles.length !== files.length) {
    if (installedFiles.length > 0) {
      console.warn(
        `  [INCOMPLETE]  ${entry.displayName} - ${installedFiles.length}/${files.length} file(s) present`,
      );
      skippedCount++;
    }
  } else if (actualSize < minSize) {
    console.warn(
      `  [TRUNCATED]   ${entry.displayName} - ` +
        `${(actualSize / 1e9).toFixed(2)} GB of expected ${(entry.sizeBytes / 1e9).toFixed(2)} GB ` +
        `(${((actualSize / entry.sizeBytes) * 100).toFixed(1)}%)`,
    );
    skippedCount++;
  } else if (files.length > 0) {
    console.log(
      `  [OK]          ${entry.displayName} (${(actualSize / 1e9).toFixed(2)} GB, ${files.length} file(s))`,
    );
    validGgufCount++;
  }
}

for (const file of ggufFiles.filter(
  (file) => !catalogedGgufFiles.has(file.filename),
)) {
  console.log(
    `  [UNCATALOGED] ${file.filename} (${(file.size / 1e9).toFixed(2)} GB)`,
  );
  validGgufCount++;
}

const mlxEntries = catalog.filter((entry) => entryBackend(entry) === "mlx");
if (mlxEntries.length > 0) {
  const entryLabel = mlxEntries.length === 1 ? "entry" : "entries";
  console.log();
  console.log(`Found ${mlxEntries.length} MLX catalog ${entryLabel}`);
}

for (const entry of mlxEntries) {
  const storagePath = entryStoragePath(entry);
  const modelPath = storagePath ? join(modelsDir, storagePath) : undefined;

  if (!supportsMlx) {
    console.log(
      `  [UNSUPPORTED] ${entry.displayName} - MLX requires Apple Silicon macOS`,
    );
    continue;
  }

  if (!storagePath || !modelPath || !existsSync(modelPath)) {
    console.warn(`  [MISSING]     ${entry.displayName}`);
    skippedCount++;
    continue;
  }

  if (
    !statSync(modelPath).isDirectory() ||
    !looksLikeMlxModelDirectory(modelPath)
  ) {
    console.warn(
      `  [INVALID]     ${entry.displayName} - expected config.json, tokenizer.json, and .safetensors files`,
    );
    skippedCount++;
    continue;
  }

  console.log(`  [OK]          ${entry.displayName} (${storagePath})`);
  validMlxCount++;
}

console.log();
console.log(
  `Models: ${validGgufCount + validMlxCount} valid ` +
    `(${validGgufCount} GGUF, ${validMlxCount} MLX), ` +
    `${skippedCount} truncated/invalid`,
);

if (skippedCount > 0) {
  console.warn("\nWarning: invalid models will not be discovered by the app.");
  console.warn(
    "Re-download the full model files or directory to include them in the distribution.",
  );
}

if (validGgufCount + validMlxCount === 0) {
  console.error(
    "\nERROR: No valid model files found. Packaging requires at least one valid GGUF file or supported MLX model directory.",
  );
  process.exit(1);
}
