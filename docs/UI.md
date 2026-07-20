# UI Customization Guide

The UI in Noctyrn is fully customizable via the `settings/ui.toml` file. This allows you to tweak the appearance of the crosshair, health bar, ammo display, and kill feed.

## Configuration File: `settings/ui.toml`

The configuration file uses the TOML format. Colors are defined as RGBA arrays `[r, g, b, a]`, where values range from 0.0 to 1.0. Positions are in pixels.

### Crosshair

Customize the crosshair's shape, color, and behavior.

```toml
[crosshair]
color = [0.0, 1.0, 0.0, 1.0] # Green
size = 10.0                  # Length of the crosshair lines
thickness = 2.0              # Thickness of the lines
gap = 5.0                    # Initial gap from the center
dot = true                   # Show center dot
dot_size = 2.0               # Size of the center dot
```

The crosshair gap dynamically expands based on the weapon's current spread (accuracy + heat).

### Health Bar

Customize the player's health bar.

```toml
[health_bar]
position = [20.0, 20.0]      # Bottom-left corner (x, y)
size = [200.0, 20.0]         # Width and height
color = [1.0, 0.0, 0.0, 1.0] # Bar fill color (Red)
text_color = [1.0, 1.0, 1.0, 1.0] # Text color (White)
border_radius = 5.0          # Rounded corners radius
```

### Ammo UI

Customize the ammo counter.

```toml
[ammo_ui]
position = [20.0, 50.0]      # Bottom-left corner (x, y)
size = [100.0, 30.0]         # Size of the container (if applicable)
color = [1.0, 1.0, 1.0, 1.0] # Text color
border_radius = 5.0          # Rounded corners radius
```

### Kill Feed

Customize the kill feed notifications.

```toml
[kill_feed]
position = [20.0, 20.0]      # Top-left corner (x, y)
max_items = 5                # Maximum number of items shown at once
item_duration = 5.0          # How long each item stays on screen (seconds)
text_color = [1.0, 1.0, 1.0, 1.0] # Text color
background_color = [0.0, 0.0, 0.0, 0.5] # Background color
border_radius = 5.0          # Rounded corners radius
```

## Reloading Settings

Currently, the game loads these settings at startup. To see changes, you must restart the game.
