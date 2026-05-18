## OpenAction GPU Stats plugin

An OpenAction ([OpenDeck](https://github.com/nekename/OpenDeck) / [Tacto](https://tacto.rivul.us)) plugin for displaying GPU stats on Linux.

Reads from `nvidia-smi`, so it currently supports **NVIDIA GPUs only**.

#### Actions

- GPU Usage
- GPU Temperature
- GPU Memory
- GPU Power

#### Requirements

- Linux
- An NVIDIA GPU with the proprietary driver installed (`nvidia-smi` available on `PATH`)

#### Install

Download the latest release archive and load it from OpenDeck:

1. Open OpenDeck → **Plugins → Install from file**
2. Select the downloaded `dev.maugap.oagpustats.zip`
3. The actions appear under the **GPU Stats** category

#### Build from source

```sh
cargo build --release
```

The binary is produced at `target/release/oagpustats` and must be placed in the plugin directory as `oagpustats-x86_64-unknown-linux-gnu` alongside `manifest.json` and `icon.png`.
