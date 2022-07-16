# SSBH Editor Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## unreleased
### Added
* Added the ability to add the current material as a preset from the Matl Editor.
* Added the Material Presets Editor to Menu > Material presets based on the Matl Editor UI.
* Added adjb files to the files list.
* Added the Adj Editor for editing model.adjb files.
* Added validation errrors related to adjb files and RENORMAL materials.
* Added a light mode theme.
* Added the ability to duplicate materials in the Matl Editor.
* Added a camera settings menu.
* Added an application icon.
* Added links to the Github wiki for editor menus.
* Added missing parameters when editing samplers in the Matl Editor.
* Added a link to the material parameter reference to the Matl Editor.
* Added a UV test pattern option for UV debug modes to better show texture orientations.

### Changed
* Improved layout of validation error messages when hovering over a file.
* Errors when loading or saving the presets.json file now show in the application log.
* Adjusted the layout of the material editor to be more consistent.
* Changed the material preset selector to not be resizable to avoid text wrapping.
* Selected meshes and models in the mesh list now render an outline on hover.
* Changed the widget for editing float values to more clearly indicate the value and be easier to use.
* Adjusted the UI for improved layout consistency.
* Changed the Vector4 labels in the Matl Editor to be more descriptive in normal and advanced mode.
* Adjusted keyboard shortcuts on MacOS to use the command key instead of ctrl.
* The Matl editor now always shows the render pass selector.
* Wrap coordinate values outside the 0.0 to 1.0 range for UV debug modes similar to a repeat wrap mode.
* Animations are automatically associated with individual model folders and assignable from the Animations tab instead of the Files list.
* Models will show up in debug shading modes or selected outlines even if no material is properly assigned.

## 0.4.1
### Added
* Added vertex color, nor channels, and prm channels to render settings.
* Added the keyboard shortcut Ctrl+Shift+O for adding a folder to the workspace.
* Added support for cube maps and 3d textures to thumbnails and the Nutexb Viewer.
* Added the ability to edit the billboard type to the skel editor.
* Added a validation check for invalid nutexb texture formats such as a nor texture using Bc7Srgb instead of Bc7Unorm.
* Added the ability to hide fighter expressions with Meshes > Hide Expressions.
* Added an option to hide the UI panels.

### Changed
* Changed the meshes list to better show that meshes are hidden when the parent folder is hidden.
* Adjusted the file list to show a warning icon for files with validation errors instead of only in the editors.
* Modified the behavior of open folder to open folders without model files and not skip animation or texture folders.
* Reduced GPU usage while the application is minimized.

### Fixed
* Fixed a rare crash when opening Nutexb files with invalid dimensions.
* Fixed inaccurate UVs when rendering models with sprite sheet params.
* Fixed an issue where all materials had alpha testing disabled in the viewport and matl editor.
* Fixed rendered colors for the texture "/common/shader/sfxpbs/default_diffuse2".
* Fixed an issue where some emissive materials would incorrectly render as having specular shading.
* Fixed a crash when minimizing the window.

## 0.4.0
### Added
* Added skeleton and bone name rendering to render settings.
* Added basic, normals, bitangents, and albedo debug shading modes.
* Added a nutexb viewer for viewing textures.

### Changed
* Changed the material shader label to always be editable.
* Simplified the process for renaming materials.

### Fixed
* Fixed scaling of red and yellow checkerboard rendering.
* Fixed col texture blending causing incorrect albedo color on some models.
* Fixed inaccurate blending of alpha for transparent materials.
* Fixed an issue where files that failed to open displayed as missing.

## 0.3.0
### Added
* Added an option to reload files in the current workspace.
* Added keyboard shortcuts for open folder and reload workspace.
* Added the ability to hide entire folders in the mesh panel.
* Added an indication for which values are modified by an animation in the animation panel.
* Added error descriptions for missing mesh attributes to the material editor.
* Added support for saving files in the mesh editor.
* Added color channel toggles to render settings.
* Added a help menu to link to resources on GitHub.
* Added an application log menu to show program and file errors.

### Changed
* Adjusted UI elements and spacing and improved the editor UI.
* Adjusted panning and zooming speed to scale correctly with the current zoom level.
* Modified CustomVector0 in the provided presets to always allow texture alpha.
* Changed the application font for better readability and language support.
* Adjusted folder names to no longer show suffixes like "c00.0".

### Fixed
* Fixed the current frame text box resizing during animation playback.
* Fixed display of Chinese, Japanese, and Korean characters in text.
* Fixed a potential crash when opening and animating models with bone cycles.
* Fixed a potential crash when animating models with invalid nuhlpb entries.
* Fixed an issue where rendering nuhlpb constraints would lead to incorrect bone orientations.
* Fixed an issue where stage ink meshes would incorrectly show a yellow checkerboard error.
* Fixed consistency of the viewport background color.
* Improved rendering of glass materials for better rendering accuracy.

## 0.2.1
### Fixed
* Fixed check for new updates.

## 0.2.0
### Added
* Material presets can be applied from the matl editor. Add and edit presets in the `presets.json` file.
* Added the ability to delete and rename materials in the matl editor.
* Added application logging for warnings and errors.

### Changed
* Increased font contrast for improved readability.
* UI adjustments for improved usability.

### Fixed
* Fixed menus not closing properly when clicking a menu item.
* Fixed a potential crash if a file has errors on saving.
* Fixed "Add Folder to Workspace..." not updating the viewport models.

## 0.1.0
First public release!
