// TrueClean E2E smoke test (no Playwright/Tauri WebDriver required).
//
// Verifies the bare minimum a user needs to trust the app:
//   1. The release binary launches without crashing.
//   2. The process stays alive long enough to render a window.
//   3. The window has the expected title ("TrueClean").
//   4. Sidebar navigation buttons are reachable (best-effort DOM probe
//      via the dev server when available; skipped in pure-binary mode).
//
// Run:
//   node tests/e2e/smoke.mjs                 # binary smoke (default)
//   node tests/e2e/smoke.mjs --dev           # also probe the vite dev server
//
// Exit code 0 = pass, non-zero = fail. No external npm deps required.

import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { setTimeout as sleep } from "node:timers/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(__dirname, "..", "..");
const BIN = path.join(
  ROOT,
  "src-tauri",
  "target",
  "release",
  process.platform === "win32" ? "trueclean.exe" : "trueclean",
);

const EXPECTED_TITLE = "TrueClean";
const BOOT_GRACE_MS = 4000; // time we give the app to boot before checking
const MAX_UPTIME_MS = 15000; // hard kill if still running after this

const args = new Set(process.argv.slice(2));
const wantDevProbe = args.has("--dev");

function fail(msg) {
  console.error(`❌ ${msg}`);
  process.exit(1);
}

function pass(msg) {
  console.log(`✅ ${msg}`);
}

async function run() {
  console.log("— TrueClean E2E smoke test —\n");

  // --- 1. Binary exists ----------------------------------------------------
  if (!existsSync(BIN)) {
    fail(`Release binary not found at ${BIN}. Run \`pnpm tauri build\` first.`);
  }
  pass(`Release binary found: ${path.relative(ROOT, BIN)}`);

  // --- 2. Launch & stay alive ----------------------------------------------
  console.log("  launching app…");
  const child = spawn(BIN, [], {
    cwd: ROOT,
    stdio: ["ignore", "pipe", "pipe"],
    detached: false,
  });

  let crashed = false;
  let stderrBuf = "";
  child.stderr.on("data", (chunk) => {
    stderrBuf += chunk.toString();
    if (stderrBuf.length > 4096) stderrBuf = stderrBuf.slice(-4096);
  });

  child.on("exit", (code, signal) => {
    if (code !== null && code !== 0) {
      crashed = true;
      console.error(`  process exited early code=${code}`);
      console.error(`  stderr tail:\n${stderrBuf.slice(-800)}`);
    }
  });

  await sleep(BOOT_GRACE_MS);

  if (crashed) {
    fail("App crashed within the boot grace period.");
  }
  if (child.exitCode !== null || child.signalCode) {
    fail("App is no longer running after boot grace period.");
  }
  pass(`App stayed alive for ${BOOT_GRACE_MS}ms`);

  // --- 3. Window title check (macOS only; skipped elsewhere) ---------------
  if (process.platform === "darwin") {
    try {
      const { execFileSync } = await import("node:child_process");
      // AppleScript: ask System Events for the window title of TrueClean.
      const script = `
        tell application "System Events"
          set wins to name of every window of (first process whose name is "TrueClean")
          if (count of wins) > 0 then
            return item 1 of wins
          else
            return ""
          end if
        end tell`;
      const title = execFileSync("osascript", ["-e", script], {
        timeout: 5000,
      })
        .toString()
        .trim();

      if (!title) {
        fail("No window found for the TrueClean process.");
      }
      if (title !== EXPECTED_TITLE) {
        fail(`Window title mismatch: expected "${EXPECTED_TITLE}", got "${title}"`);
      }
      pass(`Window title correct: "${title}"`);
    } catch (err) {
      console.warn(`  ⚠️  window title check skipped (osascript failed: ${err.message})`);
    }
  } else {
    console.warn("  ⚠️  window title check skipped (non-macOS platform)");
  }

  // --- 4. Dev-server DOM probe (optional, --dev flag) ----------------------
  if (wantDevProbe) {
    try {
      const res = await fetch("http://localhost:1420/");
      if (!res.ok) {
        fail(`Dev server responded ${res.status} on /`);
      }
      const html = await res.text();
      if (!html.includes("<div id=\"root\">")) {
        fail("Dev server HTML missing #root mount point.");
      }
      pass("Dev server reachable, #root mount point present");

      // Best-effort: fetch the bundled JS and look for sidebar nav keys so we
      // know the navigation surface compiled into the bundle.
      const jsMatch = html.match(/src="(\/assets\/index-[^"]+\.js)"/);
      if (jsMatch) {
        const jsRes = await fetch(`http://localhost:1420${jsMatch[1]}`);
        const jsText = await jsRes.text();
        const navKeys = ["overview", "scan", "junk", "settings"];
        const missing = navKeys.filter((k) => !jsText.includes(k));
        if (missing.length === 0) {
          pass("Sidebar nav keys present in bundle (overview/scan/junk/settings)");
        } else {
          console.warn(`  ⚠️  some nav keys not found in bundle: ${missing.join(", ")}`);
        }
      }
    } catch (err) {
      console.warn(`  ⚠️  dev-server probe skipped (${err.message})`);
    }
  }

  // --- 5. Clean shutdown ---------------------------------------------------
  console.log("  shutting down app…");
  try {
    child.kill("SIGTERM");
    await sleep(1500);
    if (child.exitCode === null && !child.signalCode) {
      child.kill("SIGKILL");
    }
  } catch {
    // best-effort
  }

  console.log("\n✅ Smoke test passed.");
  process.exit(0);
}

// Hard timeout safety net: never leave a hung process behind.
setTimeout(() => {
  console.error(`\n❌ Smoke test timed out after ${MAX_UPTIME_MS}ms — killing app.`);
  process.exit(2);
}, MAX_UPTIME_MS).unref();

run().catch((err) => {
  console.error(`❌ Unexpected error: ${err}`);
  process.exit(1);
});
