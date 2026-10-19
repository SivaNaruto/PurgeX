Wiz-Clean // System Residual & Space Auditor

A high-performance, lightweight Windows system cleanup utility built in Rust using eframe / egui. Wiz-Clean detects system residuals, orphaned installation leftovers, stale caches, and locked files—offering dynamic elevation and active process hunting to reclaim disk space effortlessly.

🌟 Key Features

Dynamic Environment Resolution: Automatically detects system drives (C:, D:, etc.) and user profiles dynamically without hardcoded path dependencies.

Orphaned Residual Detection:

Scans AppData\Local, AppData\Roaming, and ProgramData for abandoned folders from uninstalled applications.

Identifies leftover Windows Installer patch backlogs (.msp, .msi).

Clears stale Windows update download caches and runtime user caches.

Process Lock Hunting & Forced Purge:

Automatically identifies processes locking target residual files/folders.

Terminates locking processes safely prior to purge operations.

Clears read-only attributes recursively before deletion.

UAC Elevation Support:

Seamlessly requests Administrator privileges (UAC prompt) when purging protected system locations.

Modern Dark UI: High-density full-width responsive grid built on egui with custom styling and direct folder opening on double-click.

🚀 Getting Started

Prerequisites

Ensure you have Rust installed on your Windows machine:

Rust: https://rustup.rs (MSVC toolchain recommended)

Build and Run

Clone or download this repository.

Open a terminal in the project directory.

Run the application in release mode:

cargo run --release


🛠️ Usage Guide

Run System Audit: Click the blue Run System Audit button to initiate disk scanning.

Review Items: Double-click any row to open the target folder directly in Windows Explorer.

Select & Purge:

Check or uncheck items you wish to delete.

Leave Request UAC Elevation (Admin Mode) checked if deleting system-level residuals.

Click Purge Selected & Hunt Locks to initiate safe process termination and permanent deletion.

⚠️ Security & Safety

Wiz-Clean includes built-in safety filters to protect critical OS components (e.g., Windows, System32, Microsoft, Packages).

Files are purged permanently. Ensure you review detected items before confirming deletion.

📄 License

This project is licensed under the MIT License - see the LICENSE file for details.