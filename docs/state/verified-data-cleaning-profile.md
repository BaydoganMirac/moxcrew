# Verified data cleaning and profile

Updated: 2026-11-05

Status: Complete; locally verified

Plan: [Task0090](../plans/0090-verified-data-cleaning-profile.md)

**Project tools → Create outputs → Clean and analyze data** now stages one exact
Project CSV in Chat without starting production. The owner chooses whether to
remove exact duplicate rows, names the numeric columns to check and selects a
cleaned CSV or editable XLSX. The native form accepts only an available data
resource and preserves the exact source and policy in the typed request.

The bounded local worker trims every CSV cell, rejects empty or duplicate
headers, optionally removes exact duplicate rows and calculates descriptive
statistics only for explicitly selected numeric columns. It emits a new CSV or
XLSX without mutating the source, reopens the emitted file and requires its
shape and values to match the cleaned matrix exactly. `analysis.json` pins the
source and output hashes, row counts, trimmed-cell and duplicate counts,
per-column missing values, selected statistics and reopen verification.

## Verification

- Deterministic core coverage rejects duplicate headers and unknown numeric
  columns with stable errors. Desktop coverage proves exact source/policy
  freezing, no direct Host command and action reachability after bounded scroll
  at 800×560 in Turkish/English and both themes.
- The retained source [source.csv](../screenshots/data-profile/source.csv) has
  four input rows, one whitespace-padded region, one missing revenue value and
  one exact duplicate row. It is 70 bytes with SHA-256
  `dbdefccae6ef4ee2a546c46dd3d5f467c73004a87f2b5e5926e702ed3cbe0e56`.
- The checked [cleaned-data.csv](../screenshots/data-profile/cleaned-data.csv) is
  63 bytes with SHA-256
  `e3c75ded0c62254869513232a06b514c11a4600d8b04602d5dd40ed20ba8148f`.
  Its retained [analysis.json](../screenshots/data-profile/analysis.json) is 720
  bytes with SHA-256
  `cced8f0440c3d23d585347961032b8e610402ef061141c1b06aac05a47c37c26`.
  The checked transform records four input rows, three output rows, one removed
  duplicate, one trimmed cell and one missing revenue value. Revenue sum is
  80,750 and units sum is 32; these are descriptive checks, not a trend claim.
- The editable [cleaned-data.xlsx](../screenshots/data-profile/cleaned-data.xlsx)
  is 1,473 bytes with SHA-256
  `66bb223d78b958a94b95d09c61aaf23adbdf9081b7bf93d845b1424885c0e39f`.
  Its retained [xlsx-analysis.json](../screenshots/data-profile/xlsx-analysis.json)
  is 718 bytes with SHA-256
  `6ffd460ddec161e6bc4382b7914ab783c3e038596904384680346d166149b18f`.
  The worker reopened its worksheet XML and matched every cleaned value.
  LibreOfficeDev 26.8 independently opened the workbook and converted it back
  to the same CSV values; the only byte difference was CSV line endings.
- The real 2560×1760 Turkish dark native capture
  [data-profile-studio-tr-dark.png](../screenshots/data-profile/data-profile-studio-tr-dark.png)
  is 480,477 bytes with SHA-256
  `22d1846ba2e3d952e92b016dd1227cd93afc22745b3cd2927c069ae0d25b6208`.
  It shows the exact source, deduplication choice, numeric columns, both output
  choices and staging action without clipping or horizontal overflow.
- The retained run used Python 3.9.6 and LibreOfficeDev 26.8. Focused core,
  desktop, retained-output and office-reader checks pass. The sequential
  all-target/all-feature workspace run passed 591 tests with 13 explicit
  real-tool, PostgreSQL or manual fixture ignores and no failures. Strict
  all-target/all-feature Clippy with warnings denied, formatting, Python syntax
  and diff checks pass.

## Limits

Deduplication is exact after trimming; it does not fuzzy-match names, infer
types, impute missing values or repair malformed business data. Statistics are
limited to explicitly selected numeric columns and do not establish causation,
significance, forecasts or business meaning. Advanced spreadsheet formatting,
formulas, live provider interpretation, Windows-native behavior, signing and
public release remain separate gates.
