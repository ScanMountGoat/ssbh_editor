# ssbh_editor
SSBH Editor is an application for viewing, editing, and validating models for Smash Ultimate. Report bugs or request new features in [issues](https://github.com/ScanMountGoat/ssbh_editor/issues). Download the program in [releases](https://github.com/ScanMountGoat/ssbh_editor/releases).

## Features
The goal of SSBH Editor is to provide a more intuitive and robust visual editing experience compared to editing JSON files from ssbh_data. 
SSBH Editor checks that binary files are correctly formatted and validates relationships between files in a model folder. This is especially 
helpful for custom model imports that may have errors in the generated model files like corrupted files or invalid vertex skin weights.

- View models, textures, skeletons, and animations from Smash Ultimate
- View the effects of transition materials like the metal box or ditto materials
- View bloom, shadows, and post processing
- View the effects of helper bone constraints. This is necessary for previewing animations for mods using the [EXO Skel](https://github.com/ssbucarlos/smash-ultimate-blender) method.
- More accurate normals when animating meshes with RENORMAL materials
- Edit formats supported by ssbh_data like numdlb, nusktb, numatb, nuhlpb, and numshb files using a more intuitive interface

## Planned Features
- Additional render settings
- View camera animations
- Improvements to performance and accuracy of ssbh_wgpu
- Improved validation for errors with models, textures, and animations
- Preview stage rendering and lighting data
- Settings to adjust UI scaling for better readability
- Improved spacing and consistency for UI

## Getting Started
Open the folder containing the model and textures by clicking File > Open Folder. Clicking on supported files in the file list will open the corresponding editor. For example, clicking the model.numatb button will open the material editor. Many of the editors have additional settings that are hidden by default. Check "Advanced Settings" to allow more control over file parameters such as deleting entries or manually editing name fields.

For previewing animations, make sure the animation folder is loaded. Most animations are stored separately from the model folder and should be added with File > Add Folder to Workspace. This also works for adding additional models to the scene. 

Animations are assigned to each folder from the animation tab. Each model folder like `mario/model/body/c00` has a set of animation slots. Select a nuanmb file from the drop down to assign the animation to that slot. Animations files are grouped by folder in the drop down. Adding slots allows for playing multiple animations. Each animation slot is rendered in order starting from Slot 0. For example, assign `a00defaulteyelid.nuanmb` to Slot 0 and `a00wait1.nuanmb` to Slot 1 to play the wait animation with blinking expression. The `model.nuanmb` file should usually be selected for Slot 0.

## System Requirements
SSBH Editor runs on newer versions of Windows, Linux, and MacOS. The model rendering provided by ssbh_wgpu requires some graphical features not supported on older devices. Windows supports Vulkan or DX12, Linux supports Vulkan, and MacOS supports Metal.

## Limitations
SSBH Editor provides a recreation of key components of Smash Ultimate's rendering engine that works well for most in game and custom models while being lightweight and portable. Perfectly recreating the in game lighting and shading for thousands of models is not a goal of this application. Some values in a file may not be editable in SSBH Editor if external applications like Blender or Photoshop can provide a better editing experience. SSBH Editor can't detect all errors that can occur with a model. Many of these issues are related to issues installing and loading modded files. Not all game files that impact the appearance of a model are used by SSBH Editor. Always perform final testing with an emulator or in game.

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
SSBH Editor stores its configuration files and material presets in a dedicated directory in the user folder. All versions of SSBH Editor after 0.5.0 use the same shared location for application files. The application will only overwrite existing config file if you explicitly save changes from within the application, so downloading a new version of SSBH Editor will not change preferences or material presets. All configuration settings are editable from within the application itself, so manually editing these files usually isn't necessary.

The exact location depends on the operating system. For Windows, this is typically `C:\Users\username\AppData\Local\ssbh_editor\data` where `username` is your user name. A quick way to find the directory on Windows is to type `%localappdata%` into the path in File Explorer, hit enter, and search for the `ssbh_editor` folder.

## Building
Prebuilt binaries are only provided for Windows and MacOS at this time. Users on Linux will need to compile from source. With version 1.60 or later of the Rust toolchain installed, run `cargo build --release`.

## Useful Tools
SSBH Editor is designed for editing existing models from imports or in game. For other steps of the mod creation process, see the tools linked below.
- [Switch-Toolbox](https://github.com/KillzXGaming/Switch-Toolbox) - application that can create and edit Nutexb files (Windows only)
- [Smash Ultimate Blender](https://github.com/ssbucarlos/smash-ultimate-blender) - addon for Blender for importing and exporting models and animations.
- [ssbh_data_json](https://github.com/ultimate-research/ssbh_lib) - command line tool to convert SSBH files to JSON for editing in a text editor

## Credits
- [egui](https://github.com/emilk/egui) - user interface
- [ssbh_data](https://github.com/ultimate-research/ssbh_lib) - file formats
- [ssbh_wgpu](https://github.com/ScanMountGoat/ssbh_wgpu) - model, animation, and texture rendering
