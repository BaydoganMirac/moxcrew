const templates = [
  ["product-feature", "Product feature", "Plan, design, implement, test, and review a product slice."],
  ["bug-fix", "Bug fix", "Diagnose, fix, verify, and review one reproducible defect."],
  ["saas-launch", "SaaS launch", "Check product, security, quality, and release readiness behind an owner gate."],
  ["design-frontend", "Design + frontend", "Define an accessible interaction and implement its responsive frontend."],
  ["security-audit", "Security audit", "Threat-model, remediate selected risks, verify, and review."],
  ["release", "Release", "Verify and prepare one rollback-safe release behind an owner gate."],
];

export const TEAM_TEMPLATES = Object.freeze(templates.map(([id, name, description]) => Object.freeze({ id, name, description })));

const phases = {
  "product-feature": [["product", "research"], ["design"], ["backend", "frontend", "fullstack"], ["qa", "security"], ["review", "docs"], ["release"]],
  "bug-fix": [["research", "product"], ["backend", "frontend", "fullstack"], ["qa", "security"], ["review", "docs"], ["release"], ["design"]],
  "saas-launch": [["product", "research"], ["design"], ["backend", "frontend", "fullstack"], ["qa", "security"], ["docs", "review"], ["release"]],
  "design-frontend": [["research", "product"], ["design"], ["frontend", "fullstack"], ["qa", "review"], ["docs"], ["backend", "security", "release"]],
  "security-audit": [["research", "product"], ["security"], ["backend", "frontend", "fullstack"], ["qa"], ["review", "docs"], ["release", "design"]],
  release: [["product", "research"], ["qa", "security"], ["docs", "review"], ["release"], ["design", "backend", "frontend", "fullstack"]],
};

export function templatePhase(templateId, role) {
  const index = phases[templateId]?.findIndex((group) => group.includes(role)) ?? -1;
  if (index < 0) throw new TypeError("role is not supported by team template");
  return index;
}

export function teamTaskCopy(templateId, pack) {
  const approvalRequired = pack.role === "release" && ["saas-launch", "release"].includes(templateId);
  return {
    title: `${pack.name}: ${templateId.replaceAll("-", " ")}`,
    instruction: `Own the ${pack.role} portion of this ${templateId.replaceAll("-", " ")} goal. Produce the declared output and leave verifiable handoff evidence for dependent agents.`,
    approvalRequired,
    asksReleaseQuestion: approvalRequired,
  };
}
