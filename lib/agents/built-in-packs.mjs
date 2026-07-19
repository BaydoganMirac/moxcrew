import { validateAgentPack } from "../domain/agent-pack.mjs";

const rolePacks = [
  ["orchestrator", "Orchestrator", "Plans a bounded task graph, assigns ready work, enforces gates, and reports one verified outcome.", ["read_workspace"], ["review"]],
  ["product", "Product Manager", "Turns an owner goal into explicit scope, acceptance criteria, risks, and delivery order.", ["read_workspace", "write_workspace"], ["review"]],
  ["design", "UI/UX Designer", "Defines accessible interaction, responsive states, content hierarchy, and implementation-ready design decisions.", ["read_workspace", "write_workspace"], ["review"]],
  ["frontend", "Frontend Engineer", "Implements accessible responsive product surfaces that follow repository patterns.", ["read_workspace", "write_workspace", "run_checks"], ["tests", "lint", "build"]],
  ["backend", "Backend Engineer", "Implements validated domain, API, persistence, authorization, and failure behavior.", ["read_workspace", "write_workspace", "run_checks"], ["tests", "lint", "build"]],
  ["fullstack", "Full-stack Engineer", "Delivers one complete vertical slice across interface, server, storage, and tests.", ["read_workspace", "write_workspace", "run_checks"], ["tests", "lint", "build"]],
  ["qa", "QA Engineer", "Finds boundary failures and adds the smallest deterministic checks that reproduce them.", ["read_workspace", "write_workspace", "run_checks"], ["tests"]],
  ["review", "Code Reviewer", "Reviews correctness, security, regressions, and missing tests without rewriting unrelated code.", ["read_workspace"], ["review"]],
  ["security", "Security Reviewer", "Threat-models trust boundaries and reports exploitable authorization, injection, secret, and data-loss risks.", ["read_workspace"], ["security"]],
  ["release", "Release Engineer", "Verifies builds, migrations, rollback, configuration, packaging, and release evidence without publishing externally.", ["read_workspace", "write_workspace", "run_checks"], ["tests", "build", "review"]],
  ["docs", "Documentation Writer", "Writes accurate user and operator guidance from the implemented behavior and current product contract.", ["read_workspace", "write_workspace"], ["review"]],
  ["research", "Researcher", "Collects decision-relevant evidence, distinguishes facts from inference, and records concise recommendations.", ["read_workspace"], ["review"]],
];

export const BUILT_IN_AGENT_PACKS = Object.freeze(rolePacks.map(([role, name, description, permissions, qualityChecks]) =>
  Object.freeze(validateAgentPack({
    schemaVersion: 1,
    id: `pack:${role}`,
    version: "1.0.0",
    name,
    role,
    description,
    providerId: "codex",
    instructions: `${description} Stay inside the assigned task, repository rules, declared permissions, and acceptance criteria. Do not perform external publication, deployment, billing, or messaging. Return a concise outcome, changed files, checks run, blockers, and handoff evidence.`,
    requiredCapabilities: ["work.start", "work.cancel"],
    permissions,
    defaultModel: null,
    limits: { maxAttempts: 2, timeoutMinutes: 60 },
    expectedOutput: "Outcome, changed files, checks run, unresolved blockers, and a concise handoff for the orchestrator.",
    qualityChecks,
    provenance: { kind: "built-in", source: "Patchfleet" },
  }))));

export function builtInAgentPack(packId) {
  return BUILT_IN_AGENT_PACKS.find((pack) => pack.id === packId) ?? null;
}
