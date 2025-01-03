# clipdir

Clipdir is a clipboard history manager for wayland. Its workflow is the same as [cliphist][0], but instead of a database it stores each clipboard entry as its own file in a directory. This makes it simple to sync the clipboard history across multiple devices.

## Features

* Store clipboard changes as files in a directory for easy syncing
* Show history using `dmenu`/`rofi`/`fzf` or similar pickers
* Support for various mime-types, including images - data integrity is preserved
* Deduplication
* Support for `CLIPBOARD_STATE` (see [man wl-clipboard][1])

## Installation

Currently, only cargo is supported: `cargo install clipdir`

## Usage

```
$ clipdir help
Usage: clipdir [OPTIONS] <COMMAND>

Commands:
  store, -s, --store    Store a clipboard entry by stdin
  list, -l, --list      List clipboard entries prefixed with their id
  decode, -d, --decode  Output a clipboard entry by dmenu stdin
  help                  Print this message or the help of the given subcommand(s)
```

### Listen for clipboard changes

```
$ clipdir store --help
Usage: clipdir {store|--store|-s} [OPTIONS]

Options:
      --state <state>
          [env: CLIPBOARD_STATE=] [default: data]
      --storage-path <storage-path>
          [env: CLIPDIR_STORAGE_PATH=] [default: /home/hashworks/.local/share/clipdir]
      --byte-limit <byte-limit>
          [env: CLIPDIR_BYTE_LIMIT=] [default: 5242880]
      --dedupe-search-limit <dedupe-search-limit>
          [env: CLIPDIR_DEDUPE_SEARCH_LIMIT=] [default: 1000]
```

Call this with your desktop environment: `wl-paste --watch clipdir store`

[`wl-paste`][1] will listen for changes to your clipboard and call `clipdir store` which will store the clipboard entry in the storage directory (`~/.local/share/clipdir` by default). If not specified otherwise it will store entries up to 5 MiB and search for duplicates in the last 1000 entries.

### List clipboard history

```
$ clipdir list --help
List clipboard entries prefixed with their id

Usage: clipdir {list|--list|-l} [OPTIONS]

Options:
      --preview-length <preview-length>
          [env: CLIPDIR_PREVIEW_LENGTH=] [default: 100]
      --storage-path <storage-path>
          [env: CLIPDIR_STORAGE_PATH=] [default: /home/hashworks/.local/share/clipdir]
```

Add a keyboard binding to your desktop environment to pipe the output of `clipdir list` to `dmenu`/`rofi`/`fzf` or whatever picker you use. Afterwards extract the clipboard entry with `clipdir decode` and pipe it to [`wl-copy`][1] to set the clipboard.

```sh
$ clipdir list | dmenu | clipdir decode | wl-copy
$ clipdir list | rofi -dmenu | clipdir decode | wl-copy
$ clipdir list | fzf --no-sort | clipdir decode | wl-copy
```

### Cleanup clipboard history

You can clean up the history by deleting files in the storage directory. Simply add one of the following commands as a systemd timer:

*Delete files older than 365 days:*
```sh
$ find ~/.local/share/clipdir -type f -mtime +365 -exec rm {} \;
```

*Only keep the last 100,000 files:*
```sh
$ find ~/.local/share/clipdir -type f | sort -r | tail -n +100001 | xargs -I {} rm {}
```

### Synd clipboard history

Use tools like [syncthing][2] to sync the storage directory across multiple devices.


[0]: https://github.com/sentriz/cliphist
[1]: https://man.archlinux.org/man/wl-clipboard
[2]: https://syncthing.net