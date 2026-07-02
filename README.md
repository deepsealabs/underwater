
## Overview

![GitHub forks](https://img.shields.io/github/forks/deepsealabs/underwater?style=social)
![GitHub stars](https://img.shields.io/github/stars/deepsealabs/underwater?style=social)
![License](https://img.shields.io/github/license/deepsealabs/underwater)

Underwater is an open‑core color correction app for macOS and iOS, designed to bring professional-grade underwater video and photo enhancement tools to the community. It provides a robust, modular engine that anyone can inspect, extend, and integrate, while premium add‑ons and hosted services remain proprietary to support development.

### Status
Early days: the color-correction engine and CLI are in active development.
There’s no macOS/iOS app yet — see [Docs/ROADMAP.md](Docs/ROADMAP.md) for
what’s built, what’s next, and why it’s sequenced that way.

### Features
- Core Color Engine (Open Source, Rust — cross-platform by construction)
	 - Manual white‑balance and color temperature adjustments
	 - Tint, exposure, contrast, saturation, and vibrance controls
	 - Support for editing RAW image formats and high-resolution 4K–8K video files
	 - GPU-accelerated rendering pipeline (via `wgpu`, targeting Metal/Vulkan/DX12)

- Plugin Architecture
	- Easily extend the engine with custom filter modules

- Premium (proprietary)
	 - Automatic one-click color correction (ML-powered)
	 - Advanced LUT packs
	 - Cloud grading services

### Getting Started
#### Requirements
 - Rust (stable), via [rustup](https://rustup.rs)

#### Installation
1. Clone the repository:
    `git clone https://github.com/deepsealabs/underwater.git`
2. Build the engine and CLI:
    `cd underwater/engine`
    `cargo build`
3. Run the CLI against an image:
    `cargo run -p underwater-cli -- input.png output.png --exposure 0.5 --saturation 0.2`

macOS/iOS apps built on this engine come later — see the roadmap.

### Open-Core Model
Underwater’s repository includes the full source for the core color‑correction engine under the Apache 2.0 license. Premium features (ML-powered one-click correction, advanced LUT packs, cloud grading services) are proprietary and available via subscription or separate plugin. Feedback and contributions to the core engine are welcome.

### Contributing
We welcome contributions to the core engine! To get started:
 1. Fork the repo. Create a feature branch: `git checkout -b
    feature/my-new-filter`. 
2. Commit your changes and push: `git commit -m "Add new filter" && git push origin feature/my-new-filter`. 
3. Open a pull request describing your changes.

### License
Underwater Core Engine is released under the `Apache License 2.0`. See LICENSE for details. Premium modules and hosted services are proprietary.

Built with ❤️ by Deep Sea Labs
