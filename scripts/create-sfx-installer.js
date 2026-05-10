// Create self-extracting installer that bundles the NSIS installer
// and ALL GGUF model files.
//
// Windows PE executables are limited to < 4 GiB. If the total payload
// fits under that limit, a single self-contained .exe is produced.
// Otherwise, an external-payload pair is produced:
//
//   ModuTone_<ver>_x64-setup.exe  — small SFX launcher (~2 MB)
//   ModuTone_<ver>_x64-setup.7z   — payload archive (no size limit)
//
// The user runs setup.exe. It finds the companion .7z or embedded
// payload, extracts it to temp with an installed or companion 7-Zip
// executable, and runs the NSIS installer. The NSIS POSTINSTALL hook
// copies model files from the extracted directory into the install dir.
//
// File layout (self-contained SFX, when payload fits):
//   [64-bit Rust SFX stub PE]
//   [7z archive]
//   [8-byte LE uint64: offset where the 7z archive starts]
//
// Prerequisites:
//   npm run build                              (produces the NSIS installer)
//   7-Zip installed                            (C:\Program Files\7-Zip\7z.exe)
//   tools/sfx-stub/target/release/sfx-stub.exe (64-bit SFX stub)
//     Default stub builds need 7-Zip installed or a companion 7za.exe/7z.exe
//     next to the launcher at install time. Build the stub with
//     `--features embedded-7za` to embed tools/7za.exe when that file is
//     available locally.
//
// Usage:
//   node scripts/create-sfx-installer.js

import { execFileSync } from "node:child_process";
import {
  appendFileSync,
  copyFileSync,
  createReadStream,
  createWriteStream,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  renameSync,
  rmSync,
  statSync,
} from "node:fs";
import { join, dirname } from "node:path";
import { pipeline } from "node:stream/promises";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, "..");

// ── Configuration ──────────────────────────────────────────────────
const SFX_STUB = join(
  root,
  "tools",
  "sfx-stub",
  "target",
  "release",
  "sfx-stub.exe",
);
const SEVEN_ZIP = process.env.SEVEN_ZIP_PATH || "C:/Program Files/7-Zip/7z.exe";
const MODELS_DIR = join(root, "src-tauri", "resources", "models");
const CATALOG_PATH = join(MODELS_DIR, "model_catalog.json");

// Windows PE file size limit (4 GiB = 2^32 bytes).
// SFX must be strictly less than this.
const PE_SIZE_LIMIT = 4_294_967_296;

// Must match INSTALLED_SIZE_THRESHOLD in model_catalog.rs
const INSTALLED_SIZE_THRESHOLD = 0.9;

function formatError(error) {
  return error instanceof Error ? error.message : String(error);
}

async function appendFileContents(sourcePath, destinationPath) {
  await pipeline(
    createReadStream(sourcePath),
    createWriteStream(destinationPath, { flags: "a" }),
  );
}

function moveFileOrCopy(sourcePath, destinationPath) {
  rmSync(destinationPath, { force: true });

  try {
    renameSync(sourcePath, destinationPath);
  } catch {
    copyFileSync(sourcePath, destinationPath);
    rmSync(sourcePath, { force: true });
  }
}

// ── Validate prerequisites ────────────────────────────────────────

if (!existsSync(SFX_STUB)) {
  console.error(`ERROR: SFX stub not found at ${SFX_STUB}`);
  console.error("Build it first: cd tools/sfx-stub && cargo build --release");
  process.exit(1);
}

if (!existsSync(SEVEN_ZIP)) {
  console.error(`ERROR: 7-Zip not found at ${SEVEN_ZIP}`);
  console.error("Install 7-Zip or set SEVEN_ZIP_PATH environment variable.");
  process.exit(1);
}

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
  console.error(`ERROR: NSIS installer not found at ${installerPath}`);
  console.error("Run 'npm run build' first to produce the NSIS installer.");
  process.exit(1);
}

// ── Find valid model files ─────────────────────────────────────────

const catalog = JSON.parse(readFileSync(CATALOG_PATH, "utf-8"));

const ggufFiles = readdirSync(MODELS_DIR)
  .filter((f) => f.toLowerCase().endsWith(".gguf"))
  .map((f) => ({
    filename: f,
    path: join(MODELS_DIR, f),
    size: statSync(join(MODELS_DIR, f)).size,
  }))
  .filter((file) => {
    const entry = catalog.find((e) => e.filename === file.filename);
    if (!entry) return file.size > 0;
    return file.size >= Math.floor(entry.sizeBytes * INSTALLED_SIZE_THRESHOLD);
  });

if (ggufFiles.length === 0) {
  console.error("ERROR: No valid GGUF model files found in " + MODELS_DIR);
  process.exit(1);
}

console.log(`Found ${ggufFiles.length} model file(s):`);
for (const f of ggufFiles) {
  console.log(`  ${f.filename} (${(f.size / 1e9).toFixed(2)} GB)`);
}

// ── Calculate total payload size ───────────────────────────────────

const stubSize = statSync(SFX_STUB).size;
const nsisSize = statSync(installerPath).size;
const totalModelSize = ggufFiles.reduce((sum, f) => sum + f.size, 0);
const estimatedSfxSize = stubSize + nsisSize + totalModelSize + 4096; // +overhead

const fitsInSingleExe = estimatedSfxSize < PE_SIZE_LIMIT;

console.log();
console.log(`NSIS installer: ${(nsisSize / 1e6).toFixed(1)} MB`);
console.log(`Total models:   ${(totalModelSize / 1e9).toFixed(2)} GB`);
console.log(`SFX stub:       ${(stubSize / 1e6).toFixed(1)} MB`);
console.log(`Estimated total: ${(estimatedSfxSize / 1e9).toFixed(2)} GB`);
console.log(
  fitsInSingleExe
    ? "Strategy: self-contained SFX (fits under 4 GiB PE limit)"
    : "Strategy: external payload (exceeds 4 GiB PE limit)",
);

// ── Output directory ───────────────────────────────────────────────

const outputDir = join(root, "target", "release", "bundle", "sfx");
mkdirSync(outputDir, { recursive: true });

// ── Stage all files ────────────────────────────────────────────────

const allFiles = [
  { src: installerPath, dest: installerName },
  ...ggufFiles.map((f) => ({ src: f.path, dest: `models/${f.filename}` })),
];

const stagingDir = join(root, "target", "sfx-staging");
rmSync(stagingDir, { recursive: true, force: true });
mkdirSync(stagingDir, { recursive: true });

console.log();
for (const { src, dest } of allFiles) {
  const destPath = join(stagingDir, dest);
  mkdirSync(dirname(destPath), { recursive: true });
  console.log(`Staging: ${dest}`);
  copyFileSync(src, destPath);
}

// ── Create 7z archive (store mode — GGUF is incompressible) ───────

const archivePath = join(stagingDir, "payload.7z");

console.log();
console.log("Creating 7z archive (store mode)...");

try {
  execFileSync(
    SEVEN_ZIP,
    ["a", "-mx0", archivePath, ...allFiles.map((f) => f.dest)],
    {
      stdio: "inherit",
      timeout: 600_000,
      cwd: stagingDir,
    },
  );
} catch (error) {
  console.error(`ERROR: 7z archive creation failed: ${formatError(error)}`);
  process.exit(1);
}

const archiveSize = statSync(archivePath).size;

// ── Assemble output ────────────────────────────────────────────────

const outputName = `ModuTone_${version}_x64-setup`;

console.log();
console.log("Assembling SFX installer...");

if (fitsInSingleExe) {
  // Self-contained: [SFX stub] [7z archive] [8-byte offset trailer]
  const outputPath = join(outputDir, `${outputName}.exe`);
  copyFileSync(SFX_STUB, outputPath);
  const archiveOffset = statSync(outputPath).size;

  try {
    await appendFileContents(archivePath, outputPath);
  } catch (error) {
    console.error(
      `ERROR: Appending archive to SFX failed: ${formatError(error)}`,
    );
    process.exit(1);
  }

  const offsetBuf = Buffer.alloc(8);
  offsetBuf.writeBigUInt64LE(BigInt(archiveOffset));
  appendFileSync(outputPath, offsetBuf);

  const finalSize = statSync(outputPath).size;
  if (finalSize >= PE_SIZE_LIMIT) {
    console.error(
      `ERROR: ${outputName}.exe is ${(finalSize / 1e9).toFixed(2)} GB — ` +
        `exceeds the 4 GiB PE file size limit.`,
    );
    rmSync(outputPath, { force: true });
    process.exit(1);
  }

  console.log();
  console.log("============================================================");
  console.log("SFX Installer created successfully!");
  console.log("============================================================");
  console.log(`  Output:     ${outputPath}`);
  console.log(`  Size:       ${(finalSize / 1e9).toFixed(2)} GB`);
  console.log(`  Type:       Self-contained (single .exe)`);
} else {
  // External payload: setup.exe (small) + setup.7z (large)
  const exePath = join(outputDir, `${outputName}.exe`);
  const sevenZPath = join(outputDir, `${outputName}.7z`);

  copyFileSync(SFX_STUB, exePath);

  // Move archive to output dir (rename, then copy if crossing devices).
  moveFileOrCopy(archivePath, sevenZPath);

  const exeSize = statSync(exePath).size;
  const payloadSize = statSync(sevenZPath).size;

  console.log();
  console.log("============================================================");
  console.log("SFX Installer created successfully!");
  console.log("============================================================");
  console.log(`  Launcher:   ${exePath}`);
  console.log(`  Payload:    ${sevenZPath}`);
  console.log(`  Launcher:   ${(exeSize / 1e6).toFixed(1)} MB`);
  console.log(`  Payload:    ${(payloadSize / 1e9).toFixed(2)} GB`);
  console.log(`  Type:       External payload (launcher + .7z pair)`);
  console.log();
  console.log("Distribution: both files must be in the same directory.");
  console.log("The user runs the .exe — it finds the .7z automatically.");
}

// ── Summary ────────────────────────────────────────────────────────

console.log();
console.log(`  NSIS core:  ${(nsisSize / 1e6).toFixed(1)} MB`);
console.log(
  `  Models:     ${ggufFiles.length} file(s), ${(totalModelSize / 1e9).toFixed(2)} GB`,
);
console.log();
console.log("The user runs this single .exe to install:");
console.log("  - ModuTone app + worker sidecar");
console.log("  - Model catalog");
for (const f of ggufFiles) {
  console.log(`  - ${f.filename}`);
}

// Cleanup staging
rmSync(stagingDir, { recursive: true, force: true });
console.log();
console.log("Staging directory cleaned up.");
