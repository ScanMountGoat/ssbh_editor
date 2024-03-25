# SSBH Editor Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## unreleased
### Added
* Added support for editing the near and far clip distance to the Camera Settings window.
* Added Meshes > Expand All and Meshes Collapse All menu options.
* Added the ability to move change the current frame using the arrow keys while hovering over the animation slider.

### Changed
* Changed the color space of the color picker in the Matl Editor to be linear to be consistent with the linear float values used in game.
* Adjusted animation bar to show final frame index for all loaded animations.
* Changed the appearance of the visibility toggle icons.
* Changed the Matl Editor and Preset Editor to use a side panel to select the entry to edit as well as manually reorder entries using drag and drop.
* Moved material deletion to right clicking the selectable material label in the material list panel.

### Fixed
* Fixed an issue where the Modl Editor and Mesh Editor would not correctly highlight meshes in the viewport.

## 0.9.3 - 2024-02-13
### Fixed
* Fixed an issue where models would render with color banding artifacts in the viewport.

### 0.9.2 - 2024-01-16
### Fixed
* Fixed a regression where the application would crash immediately when entering values in the material editor.

### 0.9.1 - 2024-01-11
### Added
* Added an option to adjust application zoom with Ctrl +/- or Cmd +/-. Reset the zoom with Ctrl + 0 or Cmd + 0.

### Changed
* Reduced CPU and GPU usage while the application is idle.
* Enabled text wrapping for log messages.
* Adjusted Nutexb Viewer to show the raw image color to make sRGB vs non sRGB formats render consistently.
* Camera animations will now be unloaded when clicking File > Clear Workspace.

### Fixed
* Fixed an issue on Windows where the main window would flash white on startup.
* Fixed an issue where some models would render as black due to incorrectly applying SH ambient lighting.

### 0.9.0 - 2023-10-06
### Added
* Added an option for manually setting the UI scale in preferences to fix scaling issues on some systems.
* Added playback support for lighting animations in the viewport.
* Added playback support for camera animations in the viewport.
* Added validation for mesh vertex weights with zero weight.
* Added an option for overriding the default camera values in Camera Settings.

### Fixed
* Fixed an issue where the animation file name could not be properly edited in the Modl Editor.
* Fixed an issue where clicking Viewport > Reset Camera did not immediately update the viewport.

### 0.8.4 - 2023-06-13
### Fixed
* Fixed an issue where emission only materials rendered with specular lighting.
* Fixed an issue where fog rendering would darken half the scene with no lighting loaded.

### Changed
* Save as dialogs in editors default to the currently opened file's folder and file name when saving.

### 0.8.3 - 2023-06-02
### Added
* Added support for stage fog rendering.

### Fixed
* Fixed an issue where the same folder could be loaded multiple times.
* Fixed an issue where UV animations would sometimes be incorrectly flipped.
* Fixed an issue where CustomVector8 alpha did not update properly in the viewport.
* Fixed an issue where the application would log an error if the user presets.json file was missing.
* Fixed an issue where bone names would render on top of editor windows and menus.
* Fixed an issue where CustomFloat19 did not render properly for models like Olimar's helmet.
* Fixed an issue where some anim files would not read or save with correct scaling information.

### 0.8.2 - 2023-03-05
### Added
* Added shader information for shadows and lighting to the Matl Editor.
* Added a Device Info window for showing information about the GPU currently used by SSBH Editor.
* Added default material presets with more consistent naming and additional alpha options. 
    * These presets are always available and not editable.
    * Existing material presets are still editable as before from the Preset Editor.
* Added an option to create a new material in the Preset Editor.

### Changed
* Screenshots save with transparent backgrounds when using supported formats like PNG.
* Increased width of UI elements in Skel Editor to avoid cutting off long bone names.
* Changed the Skel Editor modes to use tabs to be consistent with other editors.
* Modified preset loading behavior to not create a default presets.json file if not necessary.
* Select either Default or User tabs when applying a material preset in the Matl Editor.
* Replaced Albedo debug mode with Unlit mode that shows texture and material coloring without lighting or vertex colors.

### Fixed
* Fixed an issue where materials did not update properly in the viewport when using Matl Editor menu options.
* Fixed an issue where the floor grid would occasionally have white artifacts at the horizon.
* Fixed an issue where alpha sample to coverage did not render properly on the Metal API for MacOS.
* Fixed an issue where loading scenes with lots of materials would create too many samplers for some GPU drivers.

### 0.8.1 - 2023-01-22
### Fixed
* Fixed a regression causing crashes when loading some large mesh files.

### 0.8.0 - 2023-01-14
### Added
* Added experimental swing collision shape rendering. After adding the animation folder, assign the swing.prc and toggle visibility in the Swing tab.
* Added validation for vertex weights being normalized.
* Added support for loading folders by dragging and dropping files or folders onto the application.
* Added a list view tab to the Anim Editor for showing animation track values.
* Added validation for premultiplied shaders with a Source Color of SourceAlpha.

### Changed
* Adjusted the behavior of adding folders and reloading folders to preserve toggled model and mesh visibility.
* Adjusted the behavior of reloading folders to reapply assigned animations.
* Added a tooltip explaining why editing of some Vector4 components is disabled when unused by the shader.
* Adjusted the MeshEx Editor to highlight the selected mesh in the viewport on hover.

### Fixed
* Fixed an issue causing the MeshEx Editor to not track changes or update properly in the viewport.
* Fixed an issue where skeletal animations would not reset properly when removing all animation slots.
* Fixed flickering due to Z-fighting on double sided meshes when using debug shading modes.
* Fixed an issue where the Matl Editor would sometimes disable editing for too many Vector4 components.

### 0.7.2 - 2022-12-28
### Added
* Added an option to Render Settings to toggle an infinite floor grid in the viewport.
* Added the ability to reorder entries in the Skel Editor, Mesh Editor, and Modl Editor using drag and drop.

### Changed
* Adjusted the new release window to display the changelog for the newer releases.
* Adjusted the files list to show dedicated icons for different file formats.

### Fixed
* Fixed a crash when enabling Draw Bone Axes in Render Settings.
* Fixed an issue where hovering over an entry in the Modl Editor would sometimes outline the wrong mesh in the viewport.

### 0.7.1 - 2022-12-19
### Added
* Added an option to add missing model.numshexb or model.adjb file by right clicking a folder.
* Added an option to quickly reset the camera under Viewport > Reset Camera.
* Added a heuristic for determining if a shader uses premultiplied alpha to the Matl Editor and viewport rendering.

### Changed
* Improved tooltips for multiple editors and windows.
* Adjusted applying a material preset to preserve sampler values when possible.
* Modified the Modl Editor to outline the selected mesh in the viewport.
* Improved rendering of spritesheet animations.

### Fixed
* Fixed an issue where some compressed UV transform animations failed to parse correctly.
* Fixed the graph not showing all UV transform fields in the Anim Editor

### 0.7.0 - 2022-11-24
### Added
* Added an option to open or add recently opened folders under File > Open Recent Folder and File > Add Recent Folder to Workspace.
* Added an option in Matl Editor to delete materials unused by model.numdlb.
* Added a visual indication in the files list for files with unsaved changes.
* Added an option to hide or show all meshes in the viewport under the Meshes menu.
* Added a toggle for scaling vertex colors by their scale value like 2.0 for colorSet1.
* Added the ShaderComplexity debug mode for viewing estimated performance costs of materials.
* Added an option to change the preferred GPU backend from preferences or the `--backend` commandline flag.
* Added an option to edit model file names to the Modl Editor.

### Changed
* Modified the Camera Settings window to use degrees instead of radians for FOV.
* Improved tooltips for various editors.
* Reworked the Mesh Editor to be more compact and outline the selected mesh in the viewport.
* Reworked the Modl editor layout.
* Adjusted tooltips to only display when the mouse is not moving.
* Adjusted formatting of displayed keyboard shortcuts on menu buttons.
* Improved accuracy for animation scaling.
* Removed the incorrectly named inherit_scale field from the Anim Editor.

### Fixed
* Fixed an issue where the application would perform unecessary work while the Hlpb Editor is open.
* Fixed an issue assigning mesh objects with repeated names in the Modl Editor in advanced mode.
* Fixed an issue where colorSet5 texture blending didn't work properly on some stages.
* Fixed an issue where applying a material preset would not update immmediately in the viewport.
* Fixed an issue reading some custom anim files with uniform scaling.

### 0.6.3 - 2022-10-09
### Added
* Added an option to add and delete constraints to the Hlpb Editor
* Added validation for model.numdlb entries.
* Add basic support for rendering animations to a GIF or image sequence.

### Changed
* Improved alignment and consistenty of UI elements.
* Show a warning icon if a nutexb thumbnail fails to load.
* Adjusted disabled parameters in the Matl Editor to indicate on hover that the parameter is unused.
* Improved the display of validation errors to appear next to the effected entry or field for better readability.
* Reduced the strictness of the check for UV wrapping to avoid false positives for cases like eye textures.
* Adjusted the texture selector to prevent assigning cube maps to 2D textures and 2D textures to cube maps.
* Improved logging when importing XML material presets missing a MaterialLibrary node.

### Fixed
* Fixed an issue where the viewport failed to update after deleting materials.
* Fixed an issue where the current frame would be set to NaN when playing empty animations.
* Fixed an issue where animations could be assigned to the wrong model and fail to play in the viewport.

## 0.6.2 - 2022-09-24
### Added
* Added rendering support for CustomVector34.
* Added rendering support for CustomVector47.
* Added validation for invalid material shader labels.
* Added support for editing depth bias to the Matl Editor.
* Added an option to remove a loaded folder in the file list by right clicking and clicking remove.
* Added screenshot saving support to Viewport > Save Screenshot.

### Changed
* Moved validation warning icons to the file button itself to always show thumbnails.
* Improved default values when adding missing required parameters in the Matl Editor.
* Improved material parameter descriptions for the Matl Editor.
* Changed vector component descriptions in the Matl Editor to display on hover instead of requiring advanced mode.
* Adjusted animation looping to apply individually to each animation.

### Fixed
* Fixed an issue where adding missing color sets in the mesh editor would add each attribute twice.
* Fixed an issue where bone axes failed to render in the viewport.
* Fixed an issue where shader and mesh attribute errors didn't update in the viewport properly.
* Fixed an issue where opening a folder didn't take into account the hide expressions setting.
* Fixed an issue where the material selector layout would break on some numatb files.

## 0.6.1 - 2022-09-05
### Added
* Added a viewport background color option to preferences.
* Added a reset button to preferences.
* Added validation for not using the Repeat wrap mode when UVs are outside the 0.0 to 1.0 range.
* Added vertex skinning and mesh parenting toggles to render settings to help debug animation related issues.
* Added an option to load an entire render folder to the Stage Lighting window with File > Load Render Folder.

### Changed
* Improved performance when adding new folders to an existing workspace.
* Adjusted validation error messages for clarity and conciseness.
* Changed the Matl Editor layout to always show all parameters even without advanced settings checked.
* Removed unused SHCP information from Stage Lighting window.

### Fixed
* Fixed an issue where canceling a folder dialog would clear the existing files.
* Fixed an issue where the Lighting Window reset button didn't work properly.
* Fixed an issue where loading lighting files would create errors about loading empty paths.
* Fixed inconsistencies in sorting for opening folders vs reloading the workspace.
* Fixed an issue where resaving a skeleton would incorrectly transpose the transforms.

## 0.6.0 - 2022-08-26
### Added
* Added a graph viewer tab to the anim editor for graphing animated values over time
* Added a field of view (fov) option to the Camera Settings window.
* Added an option to match order from a numshb file in the Mesh Editor.
* Added options to manually reorder bones or match a reference nusktb in the Skel Editor.
* Added a window for viewing and editing vertex attributes to the Mesh Editor.
* Added remaining fields to the Hlpb Editor.
* Added validation errors for duplicate mesh subindices, which can lead to incorrect skin weights in game.
* Added validation errors for numshb files for missing required attributes.
* Added validation for model.adjb file entries.
* Added lighting window to Menu > Stage Lighting for loading files in the viewport for lighting, luts, and cube maps.
* Added the MeshEx Editor for editing model.numshexb files.
* Added an option to automatically hide expressions to application preferences.
* Added an option to generated missing model.adjb entries in the Adj Editor.
* Added an option to display bones in a hierarchy tree in the Skel Editor.
* Added controls to the animation bar to control playback speed and looping.

### Changed
* Changed RGBA channel toggles to use toggle buttons to be more compact than checkboxes.
* Adjusted the layout of the Hlpb Editor to be more compact.
* Improved UI layout and consistency.
* Adjusted loading a model folder to automatically load model.nuanmb animations.
* Adjusted the material selector to show an error icon next to materials with errors.

### Fixed
* Fixed an issue where hiding a model folder had no effect when using debug shading.
* Fixed an issue changing the unk type for orient constraints in the Hlpb Editor.
* Fixed an issue where saving a numshb file would sometimes result in incorrect vertex skinning in game.
* Fixed scaling of invalid attributes and invalid shader checkerboards for high resolution screens.
* Fixed an issue where a bone could be parented to itself in the Skel Editor, causing errors when saving.

## 0.5.2 - 2022-08-08
### Added
* Added an Anim Editor for editing track flags.

### Changed
* Improved application framerate and responsiveness. These changes mostly benefit devices with integrated graphics.
* Disabled editing Vector4 components in the Matl Editor that are not used by the shader.
* Improved rendering accuracy for specular and anisotropic specular.

### Fixed
* Fixed the scrollbar not appearing in the Preset Editor window
* Fixed default stage lighting values causing rim lighting to be disabled.
* Fixed a crash when opening a new folder with the Matl Editor open.
* Fixed a crash when resizing the window to be as small as possible.
* Fixed a rendering issue where CustomBoolean4 failed to toggle indirect specular in the viewport.
* Fixed an issue where some materials would incorrectly render as having specular lighting.
* Fixed an issue where validation errors failed to update when opening folders or reloading the workspace.

## 0.5.1 - 2022-07-22
### Added
* Added wireframe rendering to debug shading.
* Added bone axes rendering for showing accumulated bone world orientations.
* Show the effected meshes in the viewport when hovering over a material in the material selector.
* Added additional fields to Hlpb Editor

### Changed
* Available animations are grouped by folder in decreasing order of affinity with the model folder. This allows assigning animations from any folder.
* File > Save in editors overwrites the original file. Use File > Save As to save to a new location.
* Improved accuracy of nuhlpb rendering for orient constraints.

### Fixed
* Fixed various issues with the slider control such as clicking updating the value.
* Fixed the #replace_cubemap texture having no thumbnail in the material editor.

### Removed
* Removed the system console on Windows.

## 0.5.0 - 2022-07-17
### Added
* Added the ability to add the current material as a preset from the Matl Editor.
* Added the Material Presets Editor to Menu > Material presets based on the Matl Editor UI.
* Added adjb files to the files list.
* Added the Adj Editor for editing model.adjb files.
* Added validation errors related to adjb files and RENORMAL materials.
* Added a light mode theme.
* Added the ability to duplicate materials in the Matl Editor.
* Added a camera settings menu.
* Added an application icon.
* Added links to the Github wiki for editor menus.
* Added missing parameters when editing samplers in the Matl Editor.
* Added a link to the material parameter reference to the Matl Editor.
* Added a UV test pattern option for UV debug modes to better show texture orientations.
* Added a preferences window for selecting light or dark mode.
* Added the ability to import material presets exported from ssbh_data_json, Cross Mod, or MatLab.
* Added an option to remove duplicate materials to the Matl Editor and Preset Editor.

### Changed
* Improved layout of validation error messages when hovering over a file.
* Errors when loading or saving the presets.json file show in the application log.
* Adjusted the layout of the material editor to be more consistent.
* Changed the material preset selector to not be resizable to avoid text wrapping.
* Selected meshes and models in the mesh list render an outline on hover.
* Changed the widget for editing float values to more clearly indicate the value and be easier to use.
* Adjusted the UI for improved layout consistency.
* Changed the Vector4 labels in the Matl Editor to be more descriptive in normal and advanced mode.
* Adjusted keyboard shortcuts on MacOS to use the command key instead of ctrl.
* Always show the render pass selector in the Matl Editor.
* Wrap coordinate values outside the 0.0 to 1.0 range for UV debug modes similar to a repeat wrap mode.
* Animations are automatically associated with individual model folders and assignable from the Animations tab instead of the Files list.
* Models will show up in debug shading modes or selected outlines even if no material is properly assigned.
* Moved material presets and application settings to an application data directory. See the README for details.

## 0.4.1 - 2022-07-07
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

## 0.4.0 - 2022-06-28
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

## 0.3.0 - 2022-06-24
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

## 0.2.1 - 2022-06-10
### Fixed
* Fixed check for new updates.

## 0.2.0 - 2022-06-10
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

## 0.1.0 - 2022-06-05
First public release!
