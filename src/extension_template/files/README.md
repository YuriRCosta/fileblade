<p align="center"><img src="assets/fileblade-extension-logo.svg" alt="FileBlade extension" width="640"></p>

---

**FileBlade {{PLUGIN_NAME}}** adds a {{PLUGIN_NAME}} blade to [FileBlade](https://github.com/data-goblin/fileblade), which gives you IDE-like sidebars for Omarchy.

{{DESCRIPTION}}

> [!NOTE]
> Replace this note with what the extension is for and what it supports. Add a
> screenshot of the blade to `assets/` and show it above this note, the way the
> other FileBlade extensions do.

## Installation / Quick-start

Install FileBlade first, then this extension:

```bash
omarchy plugin add {{REPOSITORY}} --yes --enable
omarchy restart shell
```

Then put the blade in a slot from the blade settings, or from a terminal:

```bash
fileblade blade add right {{PLUGIN_ID}}/{{MODULE_ID}}
```

This concise README is human-written to convey the simple intent and purpose of this extension. Full (agent-written) docs are in [docs/agent-written/README.md](docs/agent-written/README.md); design notes in [ARCHITECTURE.md](ARCHITECTURE.md).

Building your own extension is covered in FileBlade's [EXTENSIONS.md](https://github.com/data-goblin/fileblade/blob/main/EXTENSIONS.md).

## License

MIT.
