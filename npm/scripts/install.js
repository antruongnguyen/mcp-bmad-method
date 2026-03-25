#!/usr/bin/env node

"use strict";

const https = require("https");
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");
const zlib = require("zlib");

const REPO = "antruongnguyen/mcp-bmad-method";
const BINARY_NAME = "mcp-bmad-server";
const VERSION = require("../package.json").version;

// Map Node.js platform/arch to GitHub release asset names
function getAssetName() {
  const platform = process.platform;
  const arch = process.arch;

  const map = {
    "darwin-arm64": `${BINARY_NAME}-aarch64-apple-darwin.tar.gz`,
    "darwin-x64": `${BINARY_NAME}-x86_64-apple-darwin.tar.gz`,
    "linux-x64": `${BINARY_NAME}-x86_64-unknown-linux-gnu.tar.gz`,
    "linux-arm64": `${BINARY_NAME}-aarch64-unknown-linux-gnu.tar.gz`,
  };

  const key = `${platform}-${arch}`;
  return map[key] || null;
}

function fetch(url) {
  return new Promise((resolve, reject) => {
    https
      .get(url, { headers: { "User-Agent": "mcp-bmad-server-npm" } }, (res) => {
        if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          return fetch(res.headers.location).then(resolve, reject);
        }
        if (res.statusCode !== 200) {
          return reject(new Error(`HTTP ${res.statusCode} for ${url}`));
        }
        const chunks = [];
        res.on("data", (chunk) => chunks.push(chunk));
        res.on("end", () => resolve(Buffer.concat(chunks)));
        res.on("error", reject);
      })
      .on("error", reject);
  });
}

async function downloadFromRelease() {
  const assetName = getAssetName();
  if (!assetName) {
    return false;
  }

  const tag = `v${VERSION}`;
  const url = `https://github.com/${REPO}/releases/download/${tag}/${assetName}`;

  console.log(`Downloading ${BINARY_NAME} ${tag} for ${process.platform}-${process.arch}...`);

  try {
    const tarGz = await fetch(url);
    const tar = zlib.gunzipSync(tarGz);

    // Simple tar extraction — find the binary in the tar archive
    const binary = extractFromTar(tar, BINARY_NAME);
    if (!binary) {
      console.warn("Binary not found in archive, falling back to cargo install.");
      return false;
    }

    const dest = path.join(__dirname, "..", "bin", BINARY_NAME);
    fs.writeFileSync(dest, binary);
    fs.chmodSync(dest, 0o755);
    console.log(`Installed ${BINARY_NAME} to ${dest}`);
    return true;
  } catch (err) {
    console.warn(`Download failed: ${err.message}`);
    return false;
  }
}

// Minimal tar extraction (POSIX ustar format)
function extractFromTar(tarBuffer, targetName) {
  let offset = 0;
  while (offset < tarBuffer.length - 512) {
    const header = tarBuffer.subarray(offset, offset + 512);

    // End of archive (two zero blocks)
    if (header.every((b) => b === 0)) break;

    // File name is first 100 bytes, null-terminated
    const nameEnd = header.indexOf(0);
    const name = header.subarray(0, Math.min(nameEnd, 100)).toString("utf8");

    // File size in octal at offset 124, 12 bytes
    const sizeStr = header.subarray(124, 136).toString("utf8").trim();
    const size = parseInt(sizeStr, 8) || 0;

    offset += 512; // move past header

    const basename = path.basename(name);
    if (basename === targetName && size > 0) {
      return tarBuffer.subarray(offset, offset + size);
    }

    // Advance past file content (rounded up to 512-byte blocks)
    offset += Math.ceil(size / 512) * 512;
  }
  return null;
}

function cargoInstall() {
  console.log(`No pre-built binary available. Building from source with cargo...`);
  console.log(`Running: cargo install ${BINARY_NAME} --version ${VERSION} --root .`);

  try {
    const installDir = path.join(__dirname, "..");
    execSync(`cargo install ${BINARY_NAME} --version ${VERSION} --root "${installDir}"`, {
      stdio: "inherit",
      env: { ...process.env },
    });
    // cargo install puts the binary in <root>/bin/
    console.log(`Built and installed ${BINARY_NAME} successfully.`);
    return true;
  } catch {
    console.error(
      `Failed to build from source. Please install Rust (https://rustup.rs/) and try again.`
    );
    return false;
  }
}

async function main() {
  // Try downloading pre-built binary first
  const downloaded = await downloadFromRelease();
  if (downloaded) return;

  // Fall back to cargo install
  const built = cargoInstall();
  if (!built) {
    console.error(
      "\nCould not install mcp-bmad-server binary.\n" +
        "Options:\n" +
        "  1. Install Rust (https://rustup.rs/) and reinstall this package\n" +
        "  2. Run: cargo install mcp-bmad-server\n"
    );
    // Don't fail the npm install — the binary wrapper will show a helpful error at runtime
  }
}

main();
