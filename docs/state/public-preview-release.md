# Public preview release

Status: Release candidate; publication awaiting clean native CI

Updated: 2026-11-04

The owner authorized a GitHub pre-release for Moxcrew 0.12.0 so users can install
native packages without building from source. The public surface now leads to
the versioned release page and discloses the absent Apple notarization and
Windows publisher signature.

The first candidate CI run stopped before packaging on three targets. Review
found deterministic platform/test-harness issues rather than an installer
failure: Windows executable discovery, Unix-only secure-output fixtures and
parallel local-writer/timing contention. The candidate patch fixes those limits
without broadening the provider, result-access or Cloud contracts.

Publication remains gated on a successful general CI run and all five native
matrix targets for the exact selected commit. Every archive must contain the
expected installer formats; published assets require SHA-256 checksums and a
publicly readable post-publication verification.
