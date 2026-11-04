# Artifact verification summary

Updated: 2026-11-04

Status: Complete; locally verified

Plan: [Task0065](../plans/0065-artifact-verification-summary.md)

Output Studio now turns its existing exact artifact read into explicit, bounded
evidence without adding a backend or durable schema. The verification card is
rendered only after the response matches the selected Project, work item, path
and SHA-256. It separates two claims:

- **Exact version** means the bytes read by the Host match the hash of the
  selected Project output.
- **Format inspected** means Moxcrew additionally reopened readable text,
  parsed Office sections, loaded native image bytes or parsed media streams and
  duration. Empty, malformed and unsupported previews instead say
  **Inspection limited**.

The card always names the remaining owner check. Text still needs accuracy,
completeness and wording review; Office structure does not prove layout,
clipping or typography; image bytes do not prove crop, text or brand fidelity;
media metadata does not prove full playback or audio/visual quality. Unsupported
formats must be exported or opened in a compatible application.

The implementation is localized in Turkish and English and is covered in light
and dark themes at the 800×560 minimum window. The native Turkish dark-theme
capture is [output-verification-tr-dark.png](../screenshots/artifact-verification/output-verification-tr-dark.png)
(2560×1760, SHA-256
`0bef706247d824e8611c614f9b93dcd1a99019309326aa531aa0be5686841c54`).

## Verification

- `cargo test -p patchfleet-desktop --all-features verification -- --nocapture`
  covers supported preview kinds, limited states, scope isolation, localization
  and minimum-window reachability: 2 passed.
- `cargo test -p patchfleet-desktop --all-features`: 242 passed.
- `cargo test --workspace --all-features -- --test-threads=1`: 536 passed;
  6 opt-in external-environment tests intentionally ignored.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  `cargo fmt --all -- --check` and `git diff --check` passed.

## Limits

This UI reports evidence already produced by the exact artifact read. It does
not score quality, evaluate meaning, render Office layout, decode an entire
media file, run a model or complete the forty-ability quality matrix. Windows
result access remains deferred to native Windows verification.
