# ZR Mod Manager
- Takes .zip files containing metadata and subsequent files, and extracts them
- Keeps a copy of the original data.win, as well as any mods, in an app data directory (%appdata%\Jamesthe1\NirvanaMM\data)
- Tracks the mods last used via a TOML file
- Only one patch can be chosen per dependency (mods without a patch.xdelta are excluded), if there is a patch that depends on another then it may be loaded next
- Patches any .xdelta with xdelta3 library

## How to use
- Extract the built software to any folder (liblzma.dll and libxdelta3.dll must be next to the exe)
- Run it for the first time and close it (this will not be necessary in the full release)
- Go to the app data directory and edit `config.toml`, and make it point to the folder where ZeroRanger is installed
- Add any mods in the mods folder

## How to make a mod
- If you have an xdelta file, specifically name it `patch.xdelta`
- Create a `mod.toml` file with the following:
```toml
manifest = 1

[metadata]
name = "Mod Name"
guid = "mod.guid"
version = "v1.0.0"
author = "Author or Team Name"
depends = [ # Optional
	"example.hard.dependency",
	{guid = "example.soft.dependency", soft = true}
]
```
- Create a .zip file
- Emplace all the associated files with your mod in this zip

## Stretch goals (post-v1.0):
- Manage palettes and palette packs
- Display palettes on an example with an OpenGL shader
- Manage music packs
- Side-panel that will show all the files that will be implemented (TreeView)