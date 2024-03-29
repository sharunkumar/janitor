# janitor-service

A configurable downloads organizer written in rust

## Installation / Update

```bash
cargo install janitor-service
```

## Usage

1. run the janitor binary
2. a `janitor.toml` file would be created in your downloads directory
3. update the toml file with patterns and destinations - examples entry can be used as reference - patterns are comma separated tuples

## Configuring Auto Start/Restart with SystemD (Not available on Windows)

```bash
# Generate the unit file in into the user systemd folder
janitor-service systemd > ~/.config/systemd/user/janitor.service

# Enable and start the service
systemctl --user enable janitor.service --now

# Check program output / logs
journalctl --user -u janitor.service --follow

# Update when using systemd
cargo install janitor-service
systemctl --user restart janitor.service
```

---
fun fact: this is my first ever rust app
