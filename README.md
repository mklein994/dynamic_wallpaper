# Dynamic Wallpaper

[![Build Status](https://travis-ci.com/mklein994/dynamic_wallpaper.svg?branch=master)](https://travis-ci.com/mklein994/dynamic_wallpaper)

Change your wallpaper depending on the time of day and the position of the sun.

## Requirements

- [feh](https://feh.finalrewind.org/)
- cargo, preferably through [rustup](https://rustup.rs)
- Images that are numbered sequentially, for example
  - `~/.wallpaper/mojave-wallpaper/mojave_dynamic_{1..16}.jpeg`

## How to use it

Given your coordinates and some settings for the images, it will print out which image to use depending on the position of the sun.

### Configuration

Place this in `~/.config/dynamic_wallpaper/config.toml`:

```toml
# useful for debugging; defaults to now. Needs to be in RFC3339 format.
#now = "2018-08-31T01:45:00.123456789-05:00"
lat = 12.3456
lon = -65.4321

# these are the defaults
[wallpaper]
# The total number of images
count = 16
# The image to use just as the sun appears
daybreak = 2
# The image to use just as the moon shows up
nightfall = 13
```

Here's my setup for how I use this. It uses `feh(1)` and a systemd timer.

Create a file called `~/.fehbg`, and make it executable (`chmod +x ~/.fehbg`). Put this in it:

```sh
#!/bin/sh
feh --bg-fill --no-fehbg "$HOME/.wallpaper/mojave-wallpaper/mojave_dynamic_$(~/.cargo/bin/dynamic_wallpaper).jpeg"
```

`~/.config/systemd/user/feh-wallpaper.service`:

```ini
[Unit]
Description=Dynamic wallpaper with feh

[Service]
Type=oneshot
ExecStart=%h/.fehbg
```

`~/.config/systemd/user/feh-wallpaper.timer`:

```ini
[Unit]
Description=Dynamic wallpaper with feh

[Timer]
OnBootSec=1min
OnUnitActiveSec=10min
Unit=feh-wallpaper.service

[Install]
WantedBy=default.target
```

### Running

Test it by running `~/.fehbg`. If everything works, start and enable the systemd timer.

```sh
systemctl --user daemon-reload && systemctl --user enable --now feh-wallpaper.timer
```

Inspired by the Dynamic Desktop feature in [macOS Mojave](https://www.apple.com/macos/mojave/).
