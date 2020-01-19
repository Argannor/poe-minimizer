# poe-minimizer
Minimizes your Path of Exile Window to save electricity while you're AFK.

Path of Exile uses quite a bit of CPU while you're just standing around idle in your hideout while waiting for someone 
to buy your items. When you minimize the game's window using the task manager, the CPU usage will go down to nearly 0%.

As it can be quite cumbersome to minimize PoE using the task manager this tool will automize it for you. That way you
can't even forget it :)

## Supported Platforms

- Windows 10 64bit -> see Releases for most recent binaries
- Windows 10 32bit -> you'd have to build it yourself
- Windows < 10 maybe? But you should propably update either way..

## Installation

- Download the Zip file on the releases page and extract it
- Take a look at Settings.toml (open it with a text editor of your choice)
  - Check the property log_file_location and make sure it points to your Client.txt
  - If you're playing the game in a different language then English, please have a look at
    - afk_marker
    - afk_marker_on
- run poe-minimizer.exe 

## Help / Issues

If you're having any issues, feel free to open an issue here on github.

## Technical information

On my machine this tool uses <1MB of RAM and has non measurable CPU usage. So it should not impact your FPS.
 
Following diagram describes the algorithm:

![activity diagram][https://github.com/Argannor/poe-minimizer/raw/master/activity.diagram.png)

## Compilation

- Only tested under Windows 10 64bit
```
cargo clean 
cargo build --release
```

