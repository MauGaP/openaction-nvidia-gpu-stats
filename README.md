## OpenAction NVIDIA GPU Stats plugin — DEPRECATED

> ⚠️ **This plugin is deprecated and the repository is archived.**
>
> Its functionality has been folded into [**MauGaP/opendeck-graphs**](https://github.com/MauGaP/opendeck-graphs),
> a fork of [`mdvictor/opendeck-graphs`](https://github.com/mdvictor/opendeck-graphs) that covers GPU usage,
> temperature, memory and power (plus CPU, RAM, disk, network and more) with configurable backgrounds,
> gradients, colors, sizes and gauge geometry.
>
> Please use that plugin instead.

---

An OpenAction ([OpenDeck](https://github.com/nekename/OpenDeck) / [Tacto](https://tacto.rivul.us)) plugin for displaying NVIDIA GPU stats on Linux.

Reads from `nvidia-smi`, so it supports **NVIDIA GPUs only**.

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
2. Select the downloaded `dev.maugap.oanvgpustats.zip`
3. The actions appear under the **NVIDIA GPU Stats** category

#### Build from source

```sh
cargo build --release
```

The binary is produced at `target/release/oanvgpustats` and must be placed in the plugin directory as `oanvgpustats-x86_64-unknown-linux-gnu` alongside `manifest.json` and `icon.png`.
