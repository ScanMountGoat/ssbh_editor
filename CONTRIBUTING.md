# Contributing Guidelines
Thank you for taking the time to help improve ssbh_editor! This document covers some guidelines for how to help with development of this application.

## Filing an Issue
Report potential bugs or request new features in [issues](https://github.com/ScanMountGoat/ssbh_editor/issues). 
Make sure to check existing issues first to avoid any duplicates. It's possible a feature or bug fix has already been implemented. 
The [changelog](https://github.com/ScanMountGoat/ssbh_editor/blob/main/CHANGELOG.md) gives a brief overview of what changed with each version, 
including unreleased changes that will be included in the next version.

ssbh_editor consists of multiple projects. Please file all bugs or feature requests on ssbh_editor and not any of its dependencies.
This helps prevent any confusion from unrelated issues appearing on other projects. 

## Submitting a Pull Request
Before submitting a PR or starting work on a new change, check to see if there is an existing issue. 
If the issue doesn't exist, make an issue first explaining what you would like changed or improved. 

Make a comment on the issue if you are interested in attempting a PR to implement a fix. 
I'll let you know if it looks doable to implement and try to give a brief overview of the necessary changes that need to be made.
The codebases for ssbh_data and ssbh_wgpu are less beginner friendly than 
ssbh_editor. If an issue on ssbh_editor requires a change to ssbh_data or ssbh_wgpu, 
ask on the ssbh_editor issue first about the difficulty of the required code changes.

Before starting work on a change, make sure your fork is up to date with the latest main branch by running `git pull --rebase upstream main`.
Try to keep changes to a single commit. If you need to make small changes to respond to feedback, you can combine commits and update the branch on your fork with `git push --force`. If the PR consists of many commits, I'll squash the commits when merging. Make sure to keep your fork up to date to avoid any complications when merginging the PR. There are numerous online resources for how to use git. Make a backup of your code folder first before trying anything unfamiliar with git in case something goes wrong.

Once you've finished the implementation on your fork, open a PR for review. It's ok if the code isn't perfect or has a few bugs. 
I'll review the code when I'm available and give any feedback on changes that need to be made before merging.

## File Icons
SSBH Editor uses a number of icons to make the application easier to understand visually. Icons are an additional way for users to identify UI elements and should never be used on their own without a label or tooltip text. All icons should use the SVG vector graphics format for high quality rendering at various monitor resolution and scaling settings. Icons should render correctly on monitors with no scaling applied like a 1920x1080 monitor at 24 inches with 100% scaling in the operating system's scaling settings. The base size for most icons at this monitor DPI is 16x16 and is also the recommended canvas size for SVG files. The SVG files are rasterized in SSBH Editor at a higher resolution as needed. There are a number of applications that can create SVG files. Inkscape is free and has a convenient menu option for previewing how the icon will appear at different resolutions like 16x16 or 32x32 by clicking View > Icon Preview.
