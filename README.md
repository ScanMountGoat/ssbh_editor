# ssbh_editor [![GitHub release (latest by date including pre-releases)](https://img.shields.io/github/v/release/ScanMountGoat/ssbh_editor?include_prereleases)](https://github.com/ScanMountGoat/ssbh_editor/releases/latest) [![wiki](https://img.shields.io/badge/wiki-guide-success)](https://github.com/ScanMountGoat/ssbh_editor/wiki) [![Github All Releases](https://img.shields.io/github/downloads/ScanMountGoat/ssbh_editor/total.svg)](https://github.com/ScanMountGoat/ssbh_editor/releases/latest)

![ssbh1](https://github.com/ScanMountGoat/ssbh_editor/assets/23301691/e890beb3-54b3-4dff-8ece-0d5ed6b1b8b7)

SSBH Editor is an application for viewing, editing, and validating models for Smash Ultimate.

Check out [discussions](https://github.com/ScanMountGoat/ssbh_editor/discussions) for reading announcements, asking questions, or suggesting new features. Report bugs in [issues](https://github.com/ScanMountGoat/ssbh_editor/issues). Download the program in [releases](https://github.com/ScanMountGoat/ssbh_editor/releases).

## Features
SSBH Editor supports a number of model file types. Some files will render in the viewport if present but need to be edited with external applications like nutexb files.

| File | Description | Edit | Viewport Rendering |
| --- | --- | --- | --- |
| Adj (adjb) | Renormal mesh adjacency | :heavy_check_mark: | :heavy_check_mark: |
| Anim (nuanmb) | Animations | :heavy_check_mark: | :heavy_check_mark: |
| Hlpb (nuhlpb) | Helper bone constraints | :heavy_check_mark: | :heavy_check_mark: |
| Matl (numatb) | Materials | :heavy_check_mark: | :heavy_check_mark: |
| Mesh (numshb) | Mesh vertex data | :heavy_check_mark: | :heavy_check_mark: |
| MeshEx (numshexb) | Mesh bounding and flags | :heavy_check_mark: | :heavy_check_mark: |
| Modl (numdlb) | Mesh material assignments | :heavy_check_mark: | :heavy_check_mark: |
| Skel (nusktb) | Skeleton | :heavy_check_mark: | :heavy_check_mark: |
| Nutexb | Textures | :x: | :heavy_check_mark: |
| Xmb | LOD and model parameters | :x: | :heavy_check_mark: |
| Prc | Skeleton and swing parameters | :x: | :heavy_check_mark: |

- View models, textures, skeletons, and animations from Smash Ultimate
- View the effects of transition materials like the metal box or ditto materials
- View bloom, shadows, and post processing
- View the effects of helper bone constraints. This is necessary for previewing animations for mods using the [EXO Skel](https://github.com/ssbucarlos/smash-ultimate-blender) method.
- More accurate normals when animating meshes with RENORMAL materials
- Edit formats supported by ssbh_data like numdlb, nusktb, numatb, nuhlpb, and numshb files using a more intuitive interface

## Getting Started
Open the folder containing the model and textures by clicking File > Open Folder. Files or folders can also be dragged and dropped onto the application window to add them to the workspace. Clicking on supported files in the file list will open the corresponding editor. For example, clicking the model.numatb button will open the material editor. Many of the editors have additional settings that are hidden by default. Check "Advanced Settings" to allow more control over file parameters such as deleting entries or manually editing name fields.

For previewing animations, make sure the animation folder is loaded. Most animations are stored separately from the model folder and should be added with File > Add Folder to Workspace. This also works for adding additional models to the scene. 

Animations are assigned to each folder from the animation tab. Each model folder like `mario/model/body/c00` has a set of animation slots. Select a nuanmb file from the drop down to assign the animation to that slot. Animations files are grouped by folder in the drop down. Adding slots allows for playing multiple animations. Each animation slot is rendered in order starting from Slot 0. For example, assign `a00defaulteyelid.nuanmb` to Slot 0 and `a00wait1.nuanmb` to Slot 1 to play the wait animation with blinking expression. The `model.nuanmb` file should usually be selected for Slot 0.

## Validation
SSBH Editor provides a more intuitive and robust visual editing experience compared to editing JSON files from ssbh_data_json. SSBH Editor checks that binary files are correctly formatted and validates relationships between files in a model folder. This is helpful for custom model imports that may have errors like incorrect material names or invalid vertex skin weights. See [validation errors](https://github.com/ScanMountGoat/ssbh_editor/wiki/Validation-Errors) for details.

## Planned Features
- Additional render settings
- Improvements to performance and accuracy of ssbh_wgpu
- Improved validation for errors with models, textures, and animations
- Preview shcpanim lighting files.
- Improved spacing and consistency for UI

## System Requirements
SSBH Editor runs on newer versions of Windows, Linux, and MacOS. The model rendering provided by ssbh_wgpu requires some graphical features not supported on older devices. Windows supports Vulkan or DX12, Linux supports Vulkan, and MacOS supports Metal. SSBH Editor for Windows 
requires the Visual C++ 2015 runtime, which can be downloaded from https://www.microsoft.com/en-us/download/details.aspx?id=52685. Linux or MacOS users shouldn't need to install anything to run SSBH Editor.

## Limitations
SSBH Editor simulates key components of Smash Ultimate's rendering engine that works well for most in game and custom models while being lightweight and portable. Perfectly recreating the in game lighting and shading for every model is not a goal of this application. Not all game files that impact the appearance of a model are loaded or simulated by SSBH Editor.

Some data in files like .nuanmb or .nutexb may not be editable in SSBH Editor if external applications like Blender or Photoshop can provide a better editing experience.

SSBH Editor can't detect all errors that can occur with a model. Many of these issues are related to issues installing and loading modded files.  Always perform final testing with an emulator or in game.

SSBH Editor uses ssbh_data internally for loading and saving files. Resaving certain files without changes such as .nuanmb or .numshb files may result in a slightly different file than the original. In practice, these errors are typically small rounding errors. See the [ssbh_data docs](https://docs.rs/ssbh_data/latest/ssbh_data/) for details. If you choose to use these edited files online, you do so entirely at your own risk.

## Importing Material Presets
Cross Mod has been replaced by SSBH Editor. The material presets from Cross Mod will not work directly with SSBH Editor but can easily be converted to the right format following the steps below. 
This includes importing presets from previous versions of SSBH Editor or exported from model.numatb files.
1. Open the preset editor by clicking Menu > Material Presets.
2. Click the appropriate option under the Import menu depending on if the file is a JSON file exported from ssbh_data_json or an XML from Cross Mod and MatLab.
3. Click Material > Remove Duplicates to remove any completely identical material presets.
4. Click File > Save to save any changes to the application's presets file.

Check the application log if any error messages appear. SSBH Editor only supports the output format of the most recent version of ssbh_data_json or MatLab. 
Most errors can be fixed by exporting the .numatb file to XML or JSON again using most recent version of the programs.

## Application Configuration Files
SSBH Editor stores its configuration files and material presets in a dedicated directory in the user folder. See the [Preferences Wiki page](https://github.com/ScanMountGoat/ssbh_editor/wiki/Preferences) for details.

## Building
With a recent version of the Rust toolchain installed, run `cargo build --release`. Removing the "lto = true" line from the Cargo.toml will result in faster release builds but a slightly larger executable.

## Useful Tools
SSBH Editor is designed for editing existing models from imports or in game. For other steps of the mod creation process, see the tools linked below.
- [Smash Ultimate Blender](https://github.com/ssbucarlos/smash-ultimate-blender) - addon for Blender for importing and exporting models and animations.
- [ssbh_data_json](https://github.com/ultimate-research/ssbh_lib) - command line tool to convert SSBH files to JSON for editing in a text editor
- [Ultimate Tex](https://github.com/ScanMountGoat/ultimate_tex) - batch convert nutexb files to and from formats like DDS or PNG
- [Switch-Toolbox](https://github.com/KillzXGaming/Switch-Toolbox) - application that can create and edit Nutexb files (Windows only)

## Credits
- [egui](https://github.com/emilk/egui) - user interface
- [ssbh_data](https://github.com/ultimate-research/ssbh_lib) - file formats
- [ssbh_wgpu](https://github.com/ScanMountGoat/ssbh_wgpu) - model, animation, and texture rendering
