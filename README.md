# Grim Dawn Loot CLI
GDLC is a simple [1] command line tool to quickly find items across Grim Dawn characters. It's a vanilla solution to item storage, only helping players with figuring out what items they own and where they've placed them - which is especially useful when building characters.
The program works by decrypting the player's saves and hardcore/softcore stashes, then matches those records with Grim Dawn's game data. GDLC is read-only and doesn't write anything to disk, nor does it maintain any database of its own. 
 
[1] Decrypting the player's save files and cross-referencing the data with the database & localization files was anything but simple.

![A screenshot of the program running interactively. The results of the search terms "infiltrator's" and "aggressive gollus" are shown, with their locations. The item names and affixes are colored by their rarity.](/screenshot.png)

# Usage
* GDLC needs to be configured with the location of the game installation directory and the save files. See [configuration section below](#Configuration).
* Running `gdlc` without arguments lists all items across all characters. Any command line arguments are treated as search terms, eg. `gdlc demonic b` will find all your "Demonic Bloodsworn Scepter"'s, "Demonic Boneblade"'s, and so on.
* Invoking `gdlc -i <possible search terms>` runs the program interactively, in which case it stays open and multiple queries can be made. **This is the recommended usage** since item info is cached, which avoids reading and parsing game data on each invocation. 

# Configuration
* The installation location and save directory need to be configured.
    - For Windows: `%UserProfile%\.gdlc.conf`
    - For Linux & others: `~/.config/gdlc/gdlc.conf`
Example config:
```
installation_dir=C:\Games\Grim Dawn\
save_dir=C:\Users\<username>\My Documents\My Games\Grim Dawn\save\
```
Note that values should be without quotes and that variables are not expanded.
The string is simply split on the first '='.

# Bugs & error handling
GDLC expects files to adhere to certain formats, and might crash noisily
if it encounters unexpected data. Game updates that modify the file formats
will break this tool. 

The code has some cruft from "figuring it all out" based on others' work and there might be some mistakes in handling the game data.

# Credits
I used several other Grim Dawn tools as examples for the stash/character/database logic.
- marius00 for [Grim Dawn Item Assistant](https://github.com/marius00/iagd/).
- Aaron Hutchinson for [Grim Dawn Save Decryption](https://github.com/AaronHutchinson/Grim-Dawn-Save-Decryption/). 
The code samples were especially concise and readable.
- Chris Elison for [GDParser](https://github.com/ChrisElison/GDParser/).
