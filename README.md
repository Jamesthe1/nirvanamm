# ZR Mod Manager
- Takes .zip files containing metadata and subsequent files, and extracts them
- Keeps a copy of the original data.win, as well as any mods, in its %appdata% directory
- Tracks the patch(es) last used via a TOML file
- Only one patch can be chosen per dependency (mods without a patch.xdelta are excluded), if there is a patch that depends on another then it may be loaded next
- Patches any .xdelta with xdelta3 library

## Stretch goals (post-v1.0):
- Manage palettes and palette packs
- Display palettes on an example with an OpenGL shader
- Manage music packs
- Side-panel that will show all the files that will be implemented (TreeView)