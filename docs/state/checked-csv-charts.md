# Checked CSV charts

Updated: 2026-11-04

Status: Complete; locally verified

Plan: [Task0072](../plans/0072-checked-csv-charts.md)

**Project tools → Create outputs → Chart from CSV** now accepts one available
Project CSV, a chart title, exact category and numeric value column names, and
sum or average. It stages the typed request in Chat; it does not send, prepare or
start work.

The local data.chart worker reads only the frozen CSV copy in private scratch. It
requires a rectangular UTF-8 table with unique headers, at least one data row,
finite numeric values and no more than 40 distinct categories. It processes up
to the existing 100,000-row/200,000-cell data bound. Unsupported shapes, columns,
numbers and category counts fail explicitly; no row or category is silently
dropped.

All three outputs share one ordered aggregate:

- chart.svg is a static, standalone, accessible bar chart supporting positive
  and negative values. It contains no script, foreign object or link.
- chart-data.json records exact columns, aggregation, checked row/category
  counts and plotted values without owner paths.
- report.md repeats the same values and contributing-row counts in a concise
  table.

Owner titles, column names and category cells are escaped in SVG and Markdown.
The operation uses standard Python only and makes no model, provider, network,
account, app or Cloud call.

## Verification

- Focused core coverage proves a three-row sum, two categories, an SVG and report
  with escaped script-like cells, exact plotted values and non-numeric rejection.
- Focused desktop coverage passes at 800×560 and proves exact resource, column
  and average-aggregation staging with zero direct commands.
- Retained native evidence: [Turkish dark CSV chart form](../screenshots/data-chart/data-chart-tr-dark.png),
  SHA-256 9c1883ca497afc94535962ff9b34c8bb11f2a2016e85236804fe4289eb7e5b27.
- The full workspace passes with 547 tests and 6 explicit fixture/manual ignores:
  patchfleet-core has 112 unit tests passing with 3 ignores, and the desktop has
  249 tests passing.
- Strict Clippy, formatting and diff checks pass.

## Limits

This is deterministic grouping and sum/average charting, not statistical
interpretation. It does not infer columns, clean missing values, forecast, prove
causality or claim visual/editorial quality. Windows result access remains
deferred to Windows-PC verification.
