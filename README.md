# ssbh_editor
SSBH Editor is an application for viewing, editing, and validating models for Smash Ultimate. Report bugs or request new features in [issues](https://github.com/ScanMountGoat/ssbh_editor/issues). Download Windows releases in [releases](https://github.com/ScanMountGoat/ssbh_editor/releases).

## Features
- View models, textures, and animations from Smash Ultimate
- View the effects of transition materials like the metal box or ditto materials
- View bloom, shadows, and post processing
- View the effects of helper bone constraints. This is necessary for previewing animations for mods using the [EXO Skel](https://github.com/ssbucarlos/smash-ultimate-blender) method.
- Edit formats supported by ssbh_data like numdlb, nusktb, numatb, nuhlpb, and numshb files using a more intuitive interface

## Planned Features
- Skeleton debug display
- Additional debug shading modes
- Additional render settings
- Nutexb viewer
- View camera animations
- Improvements to performance and accuracy of ssbh_wgpu
- Improved validation for errors with models, textures, and animations
- Preview stage rendering and lighting data
- Improvements to controlling the viewport camera
- Settings to adjust UI scaling for better readability

## Getting Started
Open the folder containing the model and textures by clicking File > Open Folder. Clicking on supported files in the file list will open the corresponding editor. For example, clicking the model.numatb button will open the material editor.

For previewing animations, click the animation file in the file list to override the currently selected slot in the animation tab. Animation slots can also be added or removed from the animation tab. Animations are rendered sequentially starting from slot 0. This allows for multiple animations to play at once for adding camera animations or fighters that require more than one animation.

For opening animations from another folder, click File > Add Folder to Workspace and select the folder containing the animations. This also works for adding additional models to the scene.

## System Requirements
SSBH Editor is lightweight and does not require a powerful system to run. The application runs on newer versions of Windows, Linux, and MacOS. The model rendering provided by ssbh_wgpu requires some graphical features not supported on older devices. Windows supports Vulkan or DX12, Linux supports Vulkan, and MacOS supports Metal.

## Building
Prebuilt binaries are only provided for Windows at this time. Users on Linux or MacOS will need to compile from source. With the Rust toolchain installed, run `cargo build --release`. Include the provided `presets.json` with the compiled executable.

## Useful Tools
SSBH Editor is designed for editing existing models from imports or in game. For other steps of the mod creation process, see the tools linked below.
- [Switch-Toolbox](https://github.com/KillzXGaming/Switch-Toolbox) - application that can create and edit Nutexb files (Windows only)
- [Smash Ultimate Blender](https://github.com/ssbucarlos/smash-ultimate-blender) - addon for Blender for importing and exporting models and animations.
- [ssbh_data_json](https://github.com/ultimate-research/ssbh_lib) - command line tool to convert SSBH files to JSON for editing in a text editor

## Credits
- [egui](https://github.com/emilk/egui) - user interface
- [ssbh_data](https://github.com/ultimate-research/ssbh_lib) - file formats
- [ssbh_wgpu](https://github.com/ScanMountGoat/ssbh_wgpu) - model, animation, and texture rendering
