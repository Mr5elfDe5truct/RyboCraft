RyboCraft is open-source parametric 3D CAD in Rust: sketch, constrain, extrude, and change a
parameter to watch the timeline rebuild. [Documentation](https://github.com/Mr5elfDe5truct/RyboCraft)
·
[contributing](https://github.com/Mr5elfDe5truct/RyboCraft/blob/main/CONTRIBUTING.md) ·
[issues](https://github.com/Mr5elfDe5truct/RyboCraft/issues). AI agents can model with it over MCP:
`rybocraft-cli mcp` ([how](https://github.com/Mr5elfDe5truct/RyboCraft)).

**What's new** in RyboCraft {version} is listed below, and the
[ROADMAP](https://github.com/Mr5elfDe5truct/RyboCraft/blob/main/ROADMAP.md#known-limitations) lists
the known limitations. Designs from every earlier version open in this one.

**Which file?**
- **Windows:** `RyboCraft-{version}-windows-x64.exe` runs as is. Or install the `.msi`, or unzip the
  `-portable.zip` (x64, x86 and arm64).
- **macOS 11+** (Apple silicon and Intel): open the `.dmg` and drag `RyboCraft.app` onto Applications.
  The CLI is in `rybocraft-cli-{version}-macos-universal.zip`.
- **Linux** (x86_64 and aarch64, glibc 2.35+): the `.AppImage` (`chmod +x`, then run it), the
  `.deb` or `.rpm`, the `.flatpak` (`flatpak install --user <file>`), the `.snap`
  (`sudo snap install --dangerous <file>`, or `sudo snap install rybocraft` from the Snap Store), or the `.tar.gz`.
- **FreeBSD:** `tar -xzf rybocraft-{version}-freebsd-x86_64.tar.gz --strip-components 1 -C /usr/local`.
- **Web:** `rybocraft-web-{version}.zip` is a static site to host yourself.
- `SHA256SUMS.txt`: `sha256sum -c SHA256SUMS.txt --ignore-missing`.
- Files without a version in the name (`rybocraft-windows-x64.msi`, `rybocraft-linux-x86_64.AppImage`, …)
  are the same packages under fixed names, for download links that always fetch the latest release.

The builds aren't signed yet. On macOS, right-click `RyboCraft.app` in Applications and choose
**Open** the first time (or run `xattr -dr com.apple.quarantine /Applications/RyboCraft.app`).
On Windows, SmartScreen may say "Windows protected your PC": **More info**, then **Run anyway**.
Licences are built in (Help ▸ About ▸ Licences, `rybocraft-cli licences`); the packages also
install `THIRD-PARTY-LICENSES.txt`.

