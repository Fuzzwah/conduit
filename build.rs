//! Build script for conduit.
//!
//! When the `web` feature is enabled, this script compiles the React frontend
//! before embedding it into the binary with rust-embed.

use std::path::Path;
use std::process::Command;

fn main() {
    // Re-run build if frontend source changes
    println!("cargo::rerun-if-changed=web/src");
    println!("cargo::rerun-if-changed=web/package.json");
    println!("cargo::rerun-if-changed=web/vite.config.ts");
    println!("cargo::rerun-if-changed=web/tailwind.config.js");
    println!("cargo::rerun-if-changed=web/index.html");

    let web_dir = Path::new("web");

    // Check if web directory exists
    if !web_dir.exists() {
        println!("cargo::warning=web/ directory not found, skipping frontend build");
        return;
    }

    let dist_dir = web_dir.join("dist");

    // If node/npm are unavailable but dist/ already exists (e.g. cross-compilation
    // where assets were pre-built on the host), skip the build step entirely.
    if which::which("node").is_err() {
        if dist_dir.exists() {
            println!("cargo::warning=node not found but web/dist exists; skipping frontend build.");
            return;
        }
        println!("cargo::error=node not found. Install Node.js v18+ (https://nodejs.org/) or run scripts/preflight.sh for setup help.");
        return;
    }
    if which::which("npm").is_err() {
        if dist_dir.exists() {
            println!("cargo::warning=npm not found but web/dist exists; skipping frontend build.");
            return;
        }
        println!("cargo::error=npm not found. Install Node.js v18+ (https://nodejs.org/) or run scripts/preflight.sh for setup help.");
        return;
    }

    // Check if node_modules exists, if not run npm install
    let node_modules = web_dir.join("node_modules");
    if !node_modules.exists() {
        println!("cargo::warning=Installing frontend dependencies...");
        let status = match Command::new("npm")
            .arg("install")
            .current_dir(web_dir)
            .status()
        {
            Ok(s) => s,
            Err(e) => {
                println!("cargo::error=Failed to run npm install: {e}. Is npm on PATH?");
                return;
            }
        };

        if !status.success() {
            println!("cargo::error=npm install failed with exit code {status}. Run scripts/preflight.sh to diagnose dependency issues.");
            return;
        }
    }

    // Build the frontend
    println!("cargo::warning=Building frontend...");
    let status = match Command::new("npm")
        .arg("run")
        .arg("build")
        .current_dir(web_dir)
        .status()
    {
        Ok(s) => s,
        Err(e) => {
            println!("cargo::error=Failed to run npm build: {e}. Is npm on PATH?");
            return;
        }
    };

    if !status.success() {
        println!("cargo::error=Frontend build failed with exit code {status}.");
        return;
    }

    // Verify dist directory was created
    if !dist_dir.exists() {
        println!("cargo::error=Frontend build did not produce dist/ directory.");
        return;
    }

    println!("cargo::warning=Frontend build complete!");
}
