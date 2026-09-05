# FileBlade extension-system design

Status: blade modules and script actions are implemented; Omarchy registry
distribution is planned outside this repository.

## The short version

FileBlade does not need its own plugin store.

An **Omarchy plugin** is the package a person installs. A **FileBlade
extension** is something that package offers to FileBlade. A **blade module**
is a panel the person can place in a FileBlade slot.

The intended split is:

```text
Omarchy:   publish -> check -> install -> update -> enable/disable -> remove
FileBlade: discover enabled contributions -> validate their description -> show them
Plugin:    provide the actual module or action
```

That lets expansion packs use the normal Omarchy registry instead of creating
a second package manager inside FileBlade.

## Status by layer

| Layer | Status | Meaning |
| --- | --- | --- |
| Omarchy plugin package | Implemented by Quattro | The installable unit with a root `manifest.json` |
| FileBlade blade extension | Implemented | An enabled plugin can contribute one or more visual blade modules |
| FileBlade script-action extension | Implemented | An enabled plugin can add bounded commands to the file action menu |
| Omarchy hosted registry | Server built; client not launched | Planned `publisher/name` distribution, immutable versions, checks, and revocation |
| FileBlade-specific registry | Deliberately absent | Omarchy should remain the only store and installer |

The script-action extension point is not an alternative to Omarchy's
registry. It defines what an installed plugin may contribute; Omarchy still
decides how that plugin reaches the machine.

## Names and ownership

### Omarchy owns the package

Omarchy decides how a plugin is:

- named and published;
- downloaded and checked;
- installed and updated;
- enabled, disabled, or removed;
- blocked later if a published version turns out to be malicious.

Today the public installation path starts with a Git repository URL. The
[registry project](https://github.com/omacom/omarchy-plugin-registry) plans to
replace that distribution path with immutable `publisher/name` packages. The
registry's Quattro client is not shipped yet.

### FileBlade owns the extension sockets

FileBlade defines small, namespaced places in an Omarchy manifest where a
plugin can advertise a contribution:

```text
data-goblin.fileblade/blade    implemented: visual blade modules
data-goblin.fileblade/action   implemented: file-menu script actions
```

FileBlade reads these only from plugins that Omarchy knows about. Disabling a
plugin removes its contributions from FileBlade.

### The extension owns its behavior

The contributing plugin owns its QML, scripts, state, configuration, and any
background service it needs. FileBlade supplies bounded context and shared
helpers; it does not copy the extension into a second store.

## Package shape

A distributable FileBlade extension remains an ordinary schema-version-1
Omarchy plugin. The visual-module shape already works:

```json
{
  "schemaVersion": 1,
  "id": "publisher.example",
  "name": "Example FileBlade Extension",
  "version": "0.1.0",
  "kinds": ["service"],
  "entryPoints": {
    "service": "Service.qml"
  },
  "extensions": {
    "data-goblin.fileblade/blade": [
      {
        "id": "example",
        "name": "Example",
        "entry": "blades/Example.qml",
        "hostContract": 2
      }
    ]
  }
}
```

One Omarchy plugin may contribute several FileBlade modules. Their runtime IDs
are prefixed with the plugin ID, so two publishers cannot accidentally claim
the same FileBlade module name.

Quattro currently requires a recognized plugin kind and entry point. A pure
FileBlade extension therefore uses a very small `service` entry point. A
future generic "extension-only" package could remove that boilerplate, but it
is not required for the first registry version.

## Loading a blade module

1. A person installs an Omarchy plugin.
2. It starts disabled, so merely downloading it does not run it.
3. The person enables it.
4. Omarchy loads its service and announces that the enabled plugin list changed.
5. FileBlade reads the plugin's `data-goblin.fileblade/blade` entries.
6. FileBlade bounds the strings, checks the path, namespaces the ID, and checks
   `hostContract` compatibility.
7. Valid modules appear in the module picker.
8. FileBlade loads a module's QML only when a person puts it in a slot.
9. The module receives `BladeContext`, which exposes its state, location,
   settings, FileBlade services, and its provider service.

The sequence diagrams in [the extension guide](../../EXTENSIONS.md) show loading,
installation, and publishing visually.

## Module and service responsibilities

A visible module may exist in several slots or on several monitors. It should
therefore stay small and visual.

The plugin's one provider service owns shared work such as:

- watchers and scanners;
- subprocesses;
- caches;
- shared mutations;
- data used by more than one module instance.

Each module instance owns only presentation and its small per-tab state. This
prevents opening the same extension twice from starting the same background
work twice.

## Script actions

The script-action socket lets a plugin add an action-menu row without
writing extra menu QML. Its manifest describes a title, where the action is
valid, and an argument list for a bundled executable.

The intended safety boundary is:

- run an argument list directly, never build a shell command;
- for plugin actions, require the executable to be a real executable file
  inside that plugin's directory;
- read the manifest again immediately before running, rather than trusting an
  old command cached by the UI;
- bound arguments, selected paths, output, runtime, and concurrent runs;
- pass selections through a bounded environment value or a private temporary
  file;
- require confirmation when the action declares it;
- record the result in FileBlade's audit log without recording command output.

These checks reduce mistakes and common injection bugs. They do not sandbox a
plugin. An enabled schema-version-1 Omarchy plugin already runs as the desktop
user and must still be trusted.

## Compatibility without a dependency maze

There are two different compatibility numbers:

- `minOmarchyVersion` says which Omarchy version can load the package.
- `hostContract` says which FileBlade module API the extension expects.

The registry design avoids a general plugin dependency graph.
FileBlade should preserve that simplicity. A registry page can say "Requires
FileBlade" and link to it without silently installing a chain of other
plugins. If FileBlade is missing or too old, the extension remains installed
but its module cannot load.

## Distribution and updates

### Today

Omarchy clones a Git repository, validates it, installs it disabled, and later
updates that checkout from Git. FileBlade's current update notice also checks
those Git checkouts.

### Target registry flow

The planned Omarchy registry owns version selection, signatures, checksums,
receipts, updates, and revocation. A registry-installed plugin will not be a
normal Git checkout.

When that client ships, FileBlade should ask Omarchy for update status or send
the person to Omarchy's update flow. FileBlade's Git-based checker should then
remain only for explicit development installs. FileBlade must not grow a
second registry updater.

## Registry integration

1. Preserve namespaced `extensions` data in a published package.
2. Let the registry index enough of that data to display "Extends FileBlade"
   and help people find expansion packs.
3. Represent "Requires FileBlade" as a direct host relationship or display
   hint, not a transitive package dependency system.
4. Decide whether extension-only packages keep the tiny service entry point or
   later gain a generic package shape of their own.
5. FileBlade bundles its verified static backend. Installation needs no build
   hooks or first-run downloads; bundled executables require marketplace review.
6. Expose one Omarchy-owned update/status route that FileBlade can display
   without reimplementing registry logic.

## Explicit non-goals

- no FileBlade-only store;
- no nested package manager;
- no automatic transitive dependency installation;
- no claim that schema-version-1 plugins are sandboxed;
- no install-time scripts or surprise package installation;
- no FileBlade-specific changes required in the registry's download format.

## Open work

- Align extension metadata, host relationships, and update status with the
  registry contract.
- Exercise the real registry install and update flow when its Quattro client
  ships.
