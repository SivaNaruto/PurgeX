OrphanCleaner // High-Performance Windows System Residual & Process Auditor

OrphanCleaner is a standalone, open-source Windows utility developed in Rust using eframe/egui. Featuring a dense, responsive tabular interface inspired by classic disk space analyzers, it audits and permanently purges system garbage, uninstalled software leftovers (such as residual AppData and ProgramData folders), Windows update download caches, and orphaned installer patch files (.msp/.msi).

Built for speed, reliability, and modern OS safety, OrphanCleaner operates with non-blocking concurrency, zero hardcoded path assumptions, and automated process lock resolution.

🚀 Key Features

Non-Blocking Multithreaded Architecture: Runs heavy disk scans and recursive filesystem deletions on background worker threads using Rust std::sync::mpsc channels, preventing GUI freeze or UI frame drops.

100% Dynamic Path Detection: Automatically detects active Windows drives (C:, D:, etc.) and system directories by querying environment variables (WINDIR, SystemDrive, USERPROFILE, ProgramData).

Process Lock Hunting (os error 32 Resolution): Automatically hunts processes holding active file locks on targeted residue directories via PowerShell queries (Get-Process) and terminates them prior to deletion.

UAC Self-Elevation Loop: Supports administrative privilege escalation using --auto-purge CLI flag loops via PowerShell RunAs verbs to clean protected system-level caches safely.

Console-Suppressed Native Binary: Configured with #![windows_subsystem = "windows"] to prevent unwanted background terminal/command prompt windows in release builds.

Embedded Binary Icon: Features a programmatic 32x32 RGBA window icon baked directly into the standalone binary without external asset dependencies.

Interactive Explorer Integration: Double-clicking any row in the scan results instantly opens and highlights the item in Windows Explorer.

Full-Width Responsive UI: Uses dynamic column scaling and custom dark visuals with explicit column headers and alternating row fills.

🛠️ Tech Stack & Architecture

Language: Rust (2021 Edition)

GUI Engine: eframe / egui (v0.29)

Threading: std::sync::mpsc (Message-passing concurrency)

OS Integration: Win32 API environment mappings, std::fs filesystem operations, PowerShell process management

📦 Building from Source

Prerequisites

Rust toolchain (1.70+ recommended): Install Rust

Target OS: Windows 10 / 11

1. Clone the Repository

git clone https://github.com/SivaNaruto/OrphanCleaner.git
cd OrphanCleaner


2. Run in Development Mode

cargo run


3. Build Production Release (Console Suppressed)

cargo build --release


The optimized executable will be located at target/release/orphan_cleaner.exe.

📁 Repository Structure

OrphanCleaner/
├── Cargo.toml          # Package metadata and dependencies (eframe/egui)
├── README.md           # Documentation & resume overview
└── src/
└── main.rs         # Multithreaded GUI app, scanner engine, process hunter


💻 Technical Resume Highlights

Systems Programming in Rust: Demonstrates ownership/borrowing mechanics, custom recursive traversal depth caps, and attribute-stripping logic (readonly permission clearing).

Concurrent I/O Design: Implemented non-blocking event-driven communication using native standard library MPSC channels between worker threads and the rendering loop.

Win32 & OS Engineering: Handles Windows-specific file locking semantics, environment variable resolution, process termination hooks, and privilege escalation workflows.

Zero-Dependency Native GUI: Built a lightweight, single-file binary standalone application without relying on heavy web runtimes like Electron.

📄 License

MIT License

Copyright (c) 2026

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.