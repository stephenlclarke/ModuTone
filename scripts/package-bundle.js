// Assemble the ModuTone distribution bundle.
//
// Creates a distribution folder containing:
//   ModuTone_1.1.0_x64-setup.exe  — NSIS installer (app + worker + catalog)
//   models/                       — GGUF model files copied at install time
//
// The NSIS installer's POSTINSTALL hook copies models/*.gguf from the
// directory containing setup.exe into the install directory.
//
// Usage:
//   node scripts/package-bundle.js
//
// Prerequisites:
//   npm run build  (produces the NSIS installer)

import {
  readFileSync,
  readdirSync,
  statSync,
  copyFileSync,
  mkdirSync,
  existsSync,
} from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, "..");
const modelsDir = join(root, "src-tauri", "resources", "models");
const catalogPath = join(modelsDir, "model_catalog.json");

// Must match INSTALLED_SIZE_THRESHOLD in model_catalog.rs
const INSTALLED_SIZE_THRESHOLD = 0.9;

// Read version from tauri.conf.json
const tauriConf = JSON.parse(
  readFileSync(join(root, "src-tauri", "tauri.conf.json"), "utf-8"),
);
const version = tauriConf.version;

// Locate NSIS installer
const nsisDir = join(root, "target", "release", "bundle", "nsis");
const installerName = `ModuTone_${version}_x64-setup.exe`;
const installerPath = join(nsisDir, installerName);

if (!existsSync(installerPath)) {
  console.error(`ERROR: Installer not found at ${installerPath}`);
  console.error("Run 'npm run build' first to produce the NSIS installer.");
  process.exit(1);
}

// Read catalog for model validation
const catalog = JSON.parse(readFileSync(catalogPath, "utf-8"));

function entryStoragePaths(entry) {
  if (Array.isArray(entry.files) && entry.files.length > 0) {
    return entry.files;
  }
  const storagePath = entry.filename ?? entry.path;
  return storagePath ? [storagePath] : [];
}

function catalogEntryForFile(filename) {
  return catalog.find((entry) => entryStoragePaths(entry).includes(filename));
}

function catalogEntryComplete(entry) {
  const files = entryStoragePaths(entry);
  const actualSize = files.reduce((total, filename) => {
    const path = join(modelsDir, filename);
    return existsSync(path) ? total + statSync(path).size : total;
  }, 0);
  return (
    files.length > 0 &&
    files.every((filename) => existsSync(join(modelsDir, filename))) &&
    actualSize >= Math.floor(entry.sizeBytes * INSTALLED_SIZE_THRESHOLD)
  );
}

// Find valid GGUF files
const ggufFiles = readdirSync(modelsDir)
  .filter((f) => f.toLowerCase().endsWith(".gguf"))
  .map((f) => ({
    filename: f,
    path: join(modelsDir, f),
    size: statSync(join(modelsDir, f)).size,
  }))
  .filter((file) => {
    const entry = catalogEntryForFile(file.filename);
    if (!entry) return file.size > 0; // uncataloged: include if non-empty
    return catalogEntryComplete(entry);
  });

if (ggufFiles.length === 0) {
  console.error(`ERROR: No valid GGUF model files found in ${modelsDir}`);
  console.error("Packaging requires at least one valid model file.");
  process.exit(1);
}

// Create distribution folder
const bundleName = `ModuTone-${version}-windows-x64`;
const bundleDir = join(root, "dist-bundle", bundleName);
const bundleModelsDir = join(bundleDir, "models");

mkdirSync(bundleModelsDir, { recursive: true });

// Copy installer
console.log(`Copying installer: ${installerName}`);
copyFileSync(installerPath, join(bundleDir, installerName));

// Copy valid model files
for (const file of ggufFiles) {
  console.log(
    `Copying model: ${file.filename} (${(file.size / 1e9).toFixed(2)} GB)`,
  );
  copyFileSync(file.path, join(bundleModelsDir, file.filename));
}

// Summary
const installerSize = statSync(installerPath).size;
const totalModelSize = ggufFiles.reduce((sum, f) => sum + f.size, 0);
const totalSize = installerSize + totalModelSize;

console.log();
console.log(`Distribution bundle created: ${bundleDir}`);
console.log(`  Installer: ${(installerSize / 1e6).toFixed(1)} MB`);
console.log(
  `  Models:    ${ggufFiles.length} file(s), ${(totalModelSize / 1e9).toFixed(2)} GB`,
);
console.log(`  Total:     ${(totalSize / 1e9).toFixed(2)} GB`);
console.log();
console.log("Contents:");
console.log(`  ${bundleName}/`);
console.log(`    ${installerName}`);
for (const file of ggufFiles) {
  console.log(`    models/${file.filename}`);
}
