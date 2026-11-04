# Task0073: Editable scenario workbooks

Status: Complete; locally verified

Updated: 2026-11-04

Authorization: The owner asked to finish the remaining non-Windows product
gaps. This card advances T4 locally without a provider/model call, account
action, installed dependency or Windows claim.

## Problem

Moxcrew can create a browser-only budget calculator and literal-cell XLSX files,
but T4 promises editable formulas whose results recalculate when the selected
scenario changes. No typed operation currently binds a frozen CSV, exact item
and scenario columns, workbook formulas and independently checked totals.

## Scope

1. Add `data.scenario` for one frozen CSV with exact item, base, upside and
   downside columns plus a three-letter currency.
2. Reject malformed, duplicate, empty, non-finite or over-limit data before
   publishing any result.
3. Produce a readable XLSX with editable input cells, a Base/Upside/Downside
   selector, fixed formulas and automatic recalculation metadata.
4. Produce scenario-data.json and report.md from the same checked numeric rows;
   neither output may contain owner paths or unescaped owner structure.
5. Add a Turkish/English native form that stages the exact request in Chat
   without sending, preparing or starting work.

## Non-goals

- No forecasting, optimization, inferred columns, macros, arbitrary formulas,
  live rates, external workbook links or model-written financial advice.
- No dependency installation, Cloud/account action or Windows-native result
  access claim.

## Acceptance

- Deterministic coverage proves input bounds, exact columns, checked totals,
  fixed editable formulas, selector validation, recalculation metadata,
  escaping and non-numeric rejection.
- An independent LibreOffice check opens the workbook and proves changing the
  scenario selector recalculates the total when the installed tool is available.
- The 800x560 native form stages the exact frozen CSV and columns with zero Host
  commands, and a retained native screenshot is inspected.
- Full workspace, strict Clippy, formatting and diff checks pass.
