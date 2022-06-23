# ssbh_editor changelog
## Unreleased
### Added
* Added an option to reload files in the current workspace.
* Added keyboard shortcuts for open folder and reload workspace.
* Added the ability to hide entire folders in the mesh panel.
* Added an indication for which values are modified by an animation in the animation panel.
* Added error descriptions for missing mesh attributes to the material editor.
* Added support for saving files in the mesh editor.

### Changed
* Adjusted UI elements and spacing and improved the editor UI.
* Adjusted panning and zooming speed to scale correctly with the current zoom level.
* Modified CustomVector0 in the provided presets to always allow texture alpha.
* Changed the application font for better readability and language support.

### Fixed
* Fixed the current frame text box resizing during animation playback.
* Fixed display of Chinese, Japanese, and Korean characters in text.
* Fixed a potential crash when opening and animating models with bone cycles.
* Fixed a potential crash when animating models with invalid nuhlpb entries.
* Fixed an issue where rendering nuhlpb constraints would lead to incorrect bone orientations.
* Fixed an issue where stage ink meshes would incorrectly show a yellow checkerboard error.
* Fixed consistency of the viewport background color.

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
