
## Overview

Underwater is an open‑core color correction app for macOS and iOS, designed to bring professional-grade underwater video and photo enhancement tools to the community. It provides a robust, modular engine that anyone can inspect, extend, and integrate, while premium add‑ons and hosted services remain proprietary to support development.

### Features
- Core Color Engine (Open Source)
	 - Manual white‑balance and color temperature adjustments
	 - Tint, exposure, contrast, saturation, and vibrance controls
	 - Support for editing RAW image formats and high-resolution 4K–8K video files
	 - GPU-accelerated rendering pipeline (where supported)
	 - Automatic one-click color correction (ML-powered)

- Plugin Architecture
	- Easily extend the engine with custom filter modules

### Getting Started
#### Requirements
 - Xcode 14 or later (macOS 12+) 
 - Swift 5.7 or later 
 - macOS 12+ or iOS 15+

#### Installation
1. Clone the repository:
    `git clone https://github.com/deepsealabs/underwater.git`
2. Open the Xcode workspace:
    `cd underwater`
    `open Underwater.xcworkspace`
3. Build and run on your target (macOS or iOS simulator/device).

### Open-Core Model
Underwater’s repository includes the full source for the core color‑correction engine under the Apache 2.0 license. Premium features (e.g., advanced LUT packs, cloud grading services) are available via subscription or separate plugin. Feedback and contributions to the core engine are welcome.

### Contributing
We welcome contributions to the core engine! To get started:
 1. Fork the repo. Create a feature branch: `git checkout -b
    feature/my-new-filter`. 
2. Commit your changes and push: `git commit -m "Add new filter" && git push origin feature/my-new-filter`. 
3. Open a pull request describing your changes.

### License
Underwater Core Engine is released under the `Apache License 2.0`. See LICENSE for details. Premium modules and hosted services are proprietary.

Built with ❤️ by Deep Sea Labs
