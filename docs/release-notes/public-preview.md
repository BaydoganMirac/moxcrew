# Windows x64 file delivery

Moxcrew 0.13.2 brings the native desktop preview to Windows x64. Open a job’s
outputs to preview a text file, save an individual file, or select several
outputs and deliver them in a ZIP with a SHA-256 manifest. Saving preserves
existing files at the destination.

Choose the setup EXE for a per-user Windows install or the MSI for a
machine-wide install. Both packages contain the same desktop executable.
macOS has separate Apple Silicon and Intel DMGs. Linux x64 and ARM64 each have
an AppImage and a Deb.

The preview is unsigned. Compare the downloaded installer with its entry in
`SHA256SUMS.txt` before opening it. To upgrade, close Moxcrew or Patchfleet
and back up its stopped data directory. A rollback to an older reader requires
the complete pre-upgrade backup.

Platform requirements, data locations and recovery steps are in the
[installation guide](https://github.com/BaydoganMirac/moxcrew/blob/v0.13.2/docs/install.md).
