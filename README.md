# ssbh_editor
SSBH Editor is an application for viewing, editing, and validating models for Smash Ultimate. Report bugs or request new features in [issues](https://github.com/ScanMountGoat/ssbh_editor/issues). Download Windows releases in [releases](https://github.com/ScanMountGoat/ssbh_editor/releases).

## Features
- View models from Smash Ultimate
- View the effects of transition materials like the metal box or ditto materials
- Bloom, shadows, and post processing
- View skeletal and material animations
- View the effects of helper bone constraints
- Edit files supported by ssbh_data using a graphical interface with most changes updating in real time in the viewport

## Planned Features
- Skeleton debug display
- Additional debug shading modes
- Nutexb viewer
- View camera animations
- Improvements to performance and accuracy of ssbh_wgpu

## System Requirements
SSBH Editor is lightweight and does not require a powerful system to run. The application runs on newer versions of Windows, Linux, and MacOS. The model rendering provided by ssbh_wgpu requires some graphical features not supported on older devices. Windows supports Vulkan or DX12, Linux supports Vulkan, and MacOS supports Metal.

## Building
Prebuilt binaries are only provided for Windows at this time. Users on Linux or MacOS will need to compile from source. With the Rust toolchain installed, run `cargo build --release`. Include the provided `presets.json` with the compiled executable.

## Credits
- [egui](https://github.com/emilk/egui) - user interface
- [ssbh_data](https://github.com/ultimate-research/ssbh_lib) - file formats
- [ssbh_wgpu](https://github.com/ScanMountGoat/ssbh_wgpu) - model, animation, and texture rendering