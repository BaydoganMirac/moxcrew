# Verified editable document

Updated: 2026-11-05

Status: Complete bounded path; semantic and Microsoft Word review pending

Plan: [Task0093](../plans/0093-verified-editable-document.md)

**Project tools → Create outputs → Düzenlenebilir belge oluştur** now stages
exact owner-approved Turkish or English text in Chat without starting production
or publishing anything. The form accepts a title, optional subtitle and one to
twenty sections; each section can contain up to twelve exact paragraphs. It is
fully localized in Turkish, remains reachable at the 800×560 minimum size in
both languages and themes, and keeps the final start action explicit.

The backward-compatible `documents.create` worker still accepts the legacy
paragraph request used by existing template, rewrite and active workflow paths.
The structured path additionally produces a deterministic editable DOCX with
native Title, Subtitle, Heading 1 and Body Text styles, canonical
`content.json`, a file-hashed `document-manifest.json` and `summary.md`. The
package is reopened before acceptance; exact paragraph text and style order,
absence of external relationships and absence of active content are verified.

## Verification

- Core coverage rejects empty sections and unexpected publication fields,
  reopens the DOCX, compares every exact text/style pair, recomputes all
  manifest hashes and proves repeat requests produce byte-identical files.
  Existing DOCX template and authenticated production workflow tests continue
  to pass with their legacy one-file result contract.
- The retained editable [document.docx](../screenshots/editable-document/document.docx)
  is 2,315 bytes with SHA-256
  `52ca8cb284cafa9f9b9bba282daf24c423001683654b6a017308ed42c50e9d27`.
  [content.json](../screenshots/editable-document/content.json) is 1,073 bytes
  with SHA-256
  `da535a9299aa5bc0947ac5123059874dd3c99907330632dea6ead00ee7f6c406`;
  [summary.md](../screenshots/editable-document/summary.md) is 302 bytes with
  SHA-256
  `4eca961c1f63071ac6a118bb267f3f94c299e752ef2ee2b3b30f9335c07fb00f`.
  The retained [document-manifest.json](../screenshots/editable-document/document-manifest.json)
  is 781 bytes with SHA-256
  `0b5bd2ab1b47e866040b093d7b329f655fa0040e5619c15e79520fb573137d40`.
- LibreOfficeDev 26.8.0.0.alpha0 opened the actual DOCX and exported a tagged,
  unencrypted, JavaScript-free one-page A4
  [PDF](../screenshots/editable-document/document.pdf). The 42,551-byte PDF has
  SHA-256
  `d0fb641777b0770f4374688f6382a9539e8640379d5dc3d94a6d717b493836cb`;
  its real [first-page render](../screenshots/editable-document/document-page-1.png)
  is 125,649 bytes with SHA-256
  `a5e7098fc1faa3df68f21e946028c653e858be14412f6c64d11326be2bb1e62d`.
  The render shows the dark title band, orange accent, blue hierarchy, exact
  Turkish headings and readable body text on one page.
- The real 2560×1760 Turkish dark native capture
  [document-studio-tr-dark.png](../screenshots/editable-document/document-studio-tr-dark.png)
  is 609,468 bytes with SHA-256
  `f97b2c93b64230d33b0f0993d3539d44f3531cf4efed23638eb1f56927f34c11`.
  It shows the D1 project, localized language/title/subtitle/section fields,
  exact no-invention/no-publication policy and the visible “İsteği sohbete
  ekle” action in the native desktop app.
- The retained generation used Python 3.9.6. Focused worker, legacy DOCX,
  desktop request and native-smoke checks pass. The sequential all-target,
  all-feature workspace run passed 599 tests with 16 explicit real-tool,
  PostgreSQL, evidence-writer or manual timing ignores and no failures. Strict
  all-target/all-feature Clippy with warnings denied, formatting, Python syntax
  and diff checks pass.

## Limits

Mox formats exact approved text; it does not write, research or validate the
meaning of the content. LibreOffice renderability is proven, but Microsoft Word,
screen-reader/manual keyboard behavior, complex pagination, print fidelity and
subjective visual quality still need human review for each real document. No
cloud upload, publication, account action, live provider call or cleanup
occurred. Windows-native behavior and signing remain separate gates.
