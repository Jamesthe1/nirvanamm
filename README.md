# NirvanaMM: A ZeroRanger Mod Manager
[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/Y8Y81CWD2K)
- Takes .zip files containing metadata and subsequent files, and extracts them
- Keeps a copy of the original game files, as well as any mods, in an app data directory (%appdata%\Jamesthe1\NirvanaMM\data)
- Tracks the mods last used
- Multiple mods can be selected if they don't override the same file, or if one depends on the other
- Patches any .xdelta with xdelta3 library

## How to use
- Extract the built software to any folder (liblzma.dll and libxdelta3.dll must be next to the exe)
	> NOTE: If you wish to change where the app data is located, create a file named `dirs.toml` like so:
	> ```toml
	> appdata = 'C:\Path\To\AppData\Dir'
	> ```
- Run it and go to options, where you can set where the game is installed (be sure to press "Save")
- Add any mods into the mods folder (can be found with the "Mods" button), and click "Refresh"
- Select a mod, then click "Patch" (this will take a while the first time, make sure your data.win hasn't been modified!)

## How to make a mod
- If you have an xdelta file, specifically name it `patch.xdelta`
- Create a `mod.toml` file with the following:
```toml
manifest = 1

[metadata]
name = "Mod Name"
guid = "mod.guid"
version = "1.0.0"	# Must follow semantic versioning (https://semver.org)
author = "Author or Team Name"
depends = [ # Optional, must be an array like so
	"example.hard.dependency:>=0.5",
	{guid = "example.hard.dependency.tabled", version="1.0"},
	{guid = "example.soft.dependency", soft = true, version="<2.0.0"}
]
```
- Create a .zip file
- Emplace all the associated files with your mod in this zip
	> NOTE: Files and folders are directly copied from the zip to the game's directory. So if there's a file at `musicpacks/my_pack/song.mp3`, it will appear the same way in the game folder.

## Stretch goals (post-v1.0):
- Profiles
- Modpacks
- Mod packaging/creation page
- Manage palettes and palette packs
- Display palettes on an example with an OpenGL shader
- Manage music packs
- Side-panel that will show all the files that will be implemented (TreeView)
