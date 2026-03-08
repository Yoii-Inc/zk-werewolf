#!/usr/bin/env node

const fs = require("fs");
const path = require("path");
const { spawnSync } = require("child_process");

const wasmPackageDir = path.resolve(__dirname, "../../mpc-algebra-wasm");
const nodeEntry = path.join(wasmPackageDir, "pkg-node", "mpc_algebra_wasm.js");

if (fs.existsSync(nodeEntry)) {
  console.log("[e2e:circuits] Found pkg-node wasm build.");
  process.exit(0);
}

console.log("[e2e:circuits] pkg-node not found. Building node-target wasm...");
const result = spawnSync("wasm-pack", ["build", "--target", "nodejs", "--out-dir", "pkg-node"], {
  cwd: wasmPackageDir,
  stdio: "inherit",
});

if (result.error) {
  if (result.error.code === "ENOENT") {
    console.error("[e2e:circuits] wasm-pack is not installed.");
    console.error(
      "[e2e:circuits] Install wasm-pack and retry: https://rustwasm.github.io/wasm-pack/installer/",
    );
  } else {
    console.error(`[e2e:circuits] Failed to run wasm-pack: ${result.error.message}`);
  }
  process.exit(1);
}

if (result.status !== 0) {
  process.exit(result.status || 1);
}

if (!fs.existsSync(nodeEntry)) {
  console.error("[e2e:circuits] Build finished but pkg-node entry file was not found.");
  process.exit(1);
}

console.log("[e2e:circuits] Built pkg-node wasm successfully.");
