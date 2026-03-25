#!/usr/bin/env node

"use strict";

const { execFileSync } = require("child_process");
const path = require("path");
const fs = require("fs");

const binaryName =
  process.platform === "win32" ? "mcp-bmad-server.exe" : "mcp-bmad-server";

// Check for binary installed by postinstall script
const localBin = path.join(__dirname, binaryName);
if (fs.existsSync(localBin)) {
  run(localBin);
} else {
  // Fall back to cargo-installed binary on PATH
  run("mcp-bmad-server");
}

function run(bin) {
  const result = require("child_process").spawnSync(bin, process.argv.slice(2), {
    stdio: "inherit",
    env: process.env,
  });

  if (result.error) {
    if (result.error.code === "ENOENT") {
      console.error(
        "Error: mcp-bmad-server binary not found.\n" +
          "Install it with: cargo install mcp-bmad-server\n" +
          "Or reinstall this package: npm install @bmad-method/mcp-server"
      );
      process.exit(1);
    }
    throw result.error;
  }

  process.exit(result.status ?? 1);
}
