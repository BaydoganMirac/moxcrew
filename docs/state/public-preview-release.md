# Public preview release

Status: Release candidate; publication awaiting clean native CI

Updated: 2026-11-04

The owner authorized a GitHub pre-release for Moxcrew 0.12.0 so users can install
native packages without building from source. The public surface now leads to
the versioned release page and discloses the absent Apple notarization.

The first candidate CI run stopped before packaging on three targets. Review
first exposed deterministic platform/test-harness issues: Windows executable
discovery, Unix-only secure-output fixtures and parallel local-writer/timing
contention. A later no-fail-fast Windows run also exposed unresolved private
storage ACL/locking and platform-dependent desktop failures. Windows is therefore
withheld rather than represented as a usable installer.

Publication remains gated on successful general CI and the four advertised
macOS/Linux native targets for the exact selected commit. The experimental
Windows job remains visible but is not a public asset. Every release archive must
contain the expected formats; published assets require SHA-256 checksums and a
publicly readable post-publication verification.
