"use strict";
const fs = require("fs");
const path = require("path");

// Maps os.platform()/os.arch() to the Rust target triple used by release builds.
function getTargetTriple() {
  const platform = process.platform;
  const arch = process.arch;
  if (platform === "win32" && arch === "x64") return "x86_64-pc-windows-msvc";
  if (platform === "win32" && arch === "arm64") return "aarch64-pc-windows-msvc";
  if (platform === "darwin" && arch === "x64") return "x86_64-apple-darwin";
  if (platform === "darwin" && arch === "arm64") return "aarch64-apple-darwin";
  if (platform === "linux" && arch === "x64") return "x86_64-unknown-linux-gnu";
  if (platform === "linux" && arch === "arm64") return "aarch64-unknown-linux-gnu";
  throw new Error(`Unsupported platform: ${platform} ${arch}`);
}

const binaryName = process.platform === "win32" ? "filament-mat-lsp.exe" : "filament-mat-lsp";
const root = path.join(__dirname, "..");
const src = path.join(root, "native", "target", "release", binaryName);
const destDir = path.join(root, "native", "bin", getTargetTriple());
const dest = path.join(destDir, binaryName);

if (!fs.existsSync(src)) {
  console.error(`Release binary not found at ${src}. Run "npm run compile:rust" first.`);
  process.exit(1);
}
fs.mkdirSync(destDir, { recursive: true });
fs.copyFileSync(src, dest);
if (process.platform !== "win32") {
  fs.chmodSync(dest, 0o755);
}
console.log(`Bundled binary -> ${dest}`);
