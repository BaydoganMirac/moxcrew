import { createContext, useCallback, useContext, useEffect, useRef, useState, type FormEvent, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { commandFromForm, type HostCommand, type FormNamespace } from "./commands";

type Session = {
  providerSessionId: string;
  status: "completed" | "failed" | "interrupted" | "running" | "unknown";
  createdAt: string | null;
  lastObservedAt: string;
  terminalAt?: string;
};

type Observation = {
  schemaVersion: 1;
  provider: {
    id: "codex" | "claude" | "gemini";
    displayName: "Codex" | "Claude Code" | "Gemini CLI";
    state: "available" | "degraded" | "unavailable";
    version: string | null;
    capabilities: { recentObservation: boolean; explicitLiveStatus: boolean };
    error?: { code: string; message: string };
  };
  observedAt: string;
  sessions: Session[];
};

type Projection = {
  schemaVersion: 2;
  observations: Observation[];
};

type WorkItem = {
  workItemId: string;
  title: string;
  instruction: string;
  providerId: "codex";
  workingDirectory: string;
  status: "queued" | "launching" | "running" | "cancelling" | "blocked" | "completed" | "failed" | "interrupted";
  createdAt: string;
  revision: number;
};

type Run = {
  runId: string;
  workItemId: string;
  ownerEpoch: string;
  providerSessionId: string;
  providerTurnId: string;
  status: "running" | "cancelling" | "completed" | "failed" | "interrupted";
  revision: number;
};

type Receipt = {
  intentId: string;
  outcome: "applied" | "rejected" | "expired" | "failed";
  reasonCode: string;
  completedAt: string;
};

type WorkProjection = {
  schemaVersion: 1;
  revision: number;
  items: WorkItem[];
  runs: Run[];
  receipts: Receipt[];
};

type Workspace = {
  workspaceId: string;
  displayName: string;
  workingDirectory: string;
  revision: number;
};

type WorkspaceProjection = {
  schemaVersion: 1;
  revision: number;
  items: Workspace[];
};

type AgentPack = {
  id: string;
  version: string;
  name: string;
  role: string;
  description: string;
  providerId: "codex";
  instructions: string;
  permissions: string[];
  requiredCapabilities: string[];
  qualityChecks: string[];
  limits: { maxAttempts: number; timeoutMinutes: number };
  expectedOutput: string;
  provenance: { kind: "built-in" | "local"; source: string };
};

type AgentTeam = {
  runtimeIssue?: string;
  revision: number;
  teamId: string;
  name: string;
  goal: string;
  workspaceId: string;
  templateId: string;
  orchestratorAgentId: string;
  status: "draft" | "active" | "waiting" | "completed" | "failed" | "cancelled" | "timed_out";
  settings: { concurrency: number; retryLimit: number; timeoutMinutes: number; failurePolicy: "stop" | "continue" };
  agents: Array<{ agentId: string; pack: AgentPack; status: string }>;
  tasks: Array<{
    taskId: string;
    title: string;
    status: string;
    agentId: string;
    attempt: number;
    maxAttempts: number;
    approvalRequired: boolean;
    question: null | { questionId: string; prompt: string; maxAnswerLength: number };
  }>;
};

type AgentTeamProjection = { schemaVersion: 1; revision: number; items: AgentTeam[] };

type CloudStatus =
  | { paired: false; error?: boolean }
  | {
    paired: true;
    cloudUrl: string;
    hostId: string;
    workspaceId: string;
    lastAttemptAt: string | null;
    lastSuccessAt: string | null;
    lastErrorCode: string | null;
  };

type Feedback = {
  tone: "success" | "warning" | "error";
  title: string;
  detail: string;
};

const WORK_FEEDBACK: Record<string, Feedback> = {
  WORK_ENQUEUED: {
    tone: "success",
    title: "Task added to the queue",
    detail: "Review it below, then start Codex when control is ready.",
  },
  WORK_REMOVED: {
    tone: "success",
    title: "Queued task removed",
    detail: "No provider work was started.",
  },
  WORK_STARTED: {
    tone: "success",
    title: "Codex is working",
    detail: "The run is owned by this Patchfleet session and can be cancelled below.",
  },
  RUN_CANCELLED: {
    tone: "success",
    title: "Run cancelled safely",
    detail: "The durable receipt confirms that Codex accepted the interruption.",
  },
  WORK_TITLE_REQUIRED: {
    tone: "error",
    title: "Add a task title",
    detail: "Use a short label you will recognize in the queue.",
  },
  WORK_INSTRUCTION_REQUIRED: {
    tone: "error",
    title: "Add an instruction",
    detail: "Tell Codex exactly what should happen inside the selected repository.",
  },
  WORKSPACE_PATH_NOT_ABSOLUTE: {
    tone: "error",
    title: "Use the full repository path",
    detail: "Run pwd inside the Git repository and paste the absolute path. Relative paths and ~ are not accepted.",
  },
  WORKSPACE_SELECTION_REQUIRED: {
    tone: "error",
    title: "Choose a project",
    detail: "Select a registered project, or open Advanced and enter one absolute Git worktree path.",
  },
  WORKSPACE_SELECTION_CONFLICT: {
    tone: "error",
    title: "Choose one project source",
    detail: "Use either the registered project selector or the Advanced path, not both.",
  },
  WORKSPACE_NOT_REGISTERED: {
    tone: "error",
    title: "That project is no longer registered",
    detail: "Refresh the console and choose a project from the current list.",
  },
  WORKSPACE_NOT_ALLOWED: {
    tone: "error",
    title: "That workspace cannot be used",
    detail: "Choose an existing Git worktree root. The filesystem root and home directory are blocked.",
  },
  PROVIDER_CONTROL_UNAVAILABLE: {
    tone: "warning",
    title: "Codex control is not ready",
    detail: "Refresh providers, then confirm a supported Codex installation is available.",
  },
  COMMAND_EXPIRED: {
    tone: "warning",
    title: "That action expired",
    detail: "The task state was not changed. Use the current action shown below.",
  },
  WORK_ITEM_EXISTS: {
    tone: "warning",
    title: "That task already exists",
    detail: "Patchfleet did not add a duplicate.",
  },
  WORK_ITEM_NOT_FOUND: {
    tone: "warning",
    title: "That task is no longer available",
    detail: "The queue changed before the action arrived. Review the current state below.",
  },
  WORK_ITEM_NOT_QUEUED: {
    tone: "warning",
    title: "That task already moved on",
    detail: "Only queued tasks can be removed or started.",
  },
  STALE_ITEM_REVISION: {
    tone: "warning",
    title: "The task changed",
    detail: "Patchfleet rejected the stale action. Use the current action shown below.",
  },
  RUN_NOT_FOUND: {
    tone: "warning",
    title: "That run is no longer available",
    detail: "Review the current task state below.",
  },
  RUN_NOT_ACTIVE: {
    tone: "warning",
    title: "That run is no longer active",
    detail: "Patchfleet did not send another cancellation.",
  },
  STALE_RUN_REVISION: {
    tone: "warning",
    title: "The run changed",
    detail: "Patchfleet rejected the stale action. Use the current action shown below.",
  },
  START_OUTCOME_UNKNOWN: {
    tone: "error",
    title: "Start outcome could not be proven",
    detail: "Patchfleet blocked the task instead of risking duplicate provider work.",
  },
  RUN_SESSION_LOST: {
    tone: "error",
    title: "Run control session was lost",
    detail: "Patchfleet stopped claiming control and preserved the failure as a durable receipt.",
  },
  PROVIDER_CONTROL_FAILED: {
    tone: "error",
    title: "Codex control failed safely",
    detail: "No success was claimed. Review the durable state and refresh providers before retrying.",
  },
  OUTCOME_PENDING: {
    tone: "warning",
    title: "Waiting for a definitive outcome",
    detail: "Refresh the console before sending another action.",
  },
  INVALID_COMMAND: {
    tone: "error",
    title: "That action could not be used",
    detail: "Check the current form values and try again from this console.",
  },
};

function time(value: string) {
  return new Intl.DateTimeFormat("en", { dateStyle: "medium", timeStyle: "short" }).format(
    new Date(value),
  );
}

function shortId(value: string) {
  return value.length > 13 ? `${value.slice(0, 8)}…${value.slice(-4)}` : value;
}

function providerLabel(state: Observation["provider"]["state"]) {
  if (state === "available") return "Available";
  if (state === "degraded") return "Degraded";
  return "Unavailable";
}

export default function App() {
  const [snapshot, setSnapshot] = useState<HostSnapshot | null>(null);
  const [loadError, setLoadError] = useState(false);
  const [busy, setBusy] = useState(false);
  const [feedback, setFeedback] = useState<Feedback | null>(null);
  const mounted = useRef(true);
  const refreshing = useRef(false);
  const acting = useRef(false);
  const lastCommand = useRef<HostCommand | null>(null);

  const refresh = useCallback(async () => {
    if (refreshing.current) return;
    refreshing.current = true;
    try {
      const next = await invoke<HostSnapshot>("host_snapshot");
      if (mounted.current) { setSnapshot(next); setLoadError(false); }
    } catch {
      if (mounted.current) setLoadError(true);
    } finally { refreshing.current = false; }
  }, []);

  useEffect(() => {
    mounted.current = true;
    void refresh();
    const timer = setInterval(() => void refresh(), 2000);
    return () => { mounted.current = false; clearInterval(timer); };
  }, [refresh]);

  const dispatch = useCallback(async (command: HostCommand) => {
    if (acting.current) return false;
    acting.current = true;
    setBusy(true);
    lastCommand.current = command;
    try {
      const receipt = await invoke<{ reasonCode?: string; outcome?: string }>("host_action", command);
      const rejected = receipt?.outcome && receipt.outcome !== "applied";
      const code = receipt?.reasonCode ?? command.action.toUpperCase();
      setFeedback(WORK_FEEDBACK[code] ?? {
        tone: rejected ? "warning" : "success",
        title: rejected ? "Action was not applied" : "Action recorded",
        detail: code.replaceAll("_", " ").toLowerCase(),
      });
      lastCommand.current = null;
      await refresh();
      return !rejected;
    } catch (error) {
      const code = typeof error === "string" && /^[A-Z][A-Z0-9_]{1,100}$/.test(error) ? error : "HOST_ACTION_FAILED";
      setFeedback(WORK_FEEDBACK[code] ?? { tone: "error", title: "The host could not confirm this action", detail: `${code}. Retry the same action to retrieve its durable outcome.` });
      await refresh();
      return false;
    } finally {
      acting.current = false;
      setBusy(false);
    }
  }, [refresh]);

  const context = { busy, dispatch, report: (detail: string) => setFeedback({tone: "error", title: "Check the form values", detail}) };
  const observed = snapshot?.observation?.observations.length ? snapshot.observation : null;
  const activated = snapshot && (snapshot.work.items.length > 0 || snapshot.work.receipts.length > 0);
  return <HostContext.Provider value={context}>
    <main className="app-shell" aria-busy={busy}>
      <header className="app-header">
        <a className="brand" href="#" aria-label="Patchfleet home"><span className="brand-mark" aria-hidden="true">P</span><span>Patchfleet</span><span className="local-label">Desktop</span></a>
        <span className="privacy-pill"><span aria-hidden="true">●</span> Local data stays local</span>
      </header>
      <section className="hero operational-hero" aria-labelledby="page-title">
        <div className="hero-copy"><p className="eyebrow">Local command center</p><h1 id="page-title">{activated ? "Local work, under control." : "Run your first controlled task."}</h1><p className="summary">Choose a local project, form a team, and keep a durable record of every action.</p></div>
        <div className="hero-actions"><a className="button-link" href="#queue-work">New task</a><HostForm namespace="observe"><button className="secondary" disabled={busy || loadError || !snapshot} type="submit">Refresh providers</button></HostForm></div>
      </section>
      {feedback ? <section className={`feedback ${feedback.tone}`} role={feedback.tone === "error" ? "alert" : "status"}><span className="feedback-mark" aria-hidden="true">{feedback.tone === "success" ? "✓" : "!"}</span><div><h2>{feedback.title}</h2><p>{feedback.detail}</p>{lastCommand.current ? <button className="secondary" disabled={busy} onClick={() => lastCommand.current && void dispatch(lastCommand.current)}>Retry same action</button> : null}</div></section> : null}
      {loadError ? <section className="notice error panel" role="alert"><h2>Local host needs attention</h2><p>The Rust host could not supply a current snapshot. Actions are disabled until it recovers.</p><button className="secondary" onClick={() => void refresh()}>Retry connection</button></section> : null}
      {!snapshot ? <section className="notice panel" role="status"><h2>{loadError ? "Host unavailable" : "Opening your local host…"}</h2><p>Patchfleet requires its desktop window to access the local Rust host.</p></section> : <>
        <fieldset className="host-content" disabled={loadError || busy}>
          <ReadinessOverview projection={observed} cloud={snapshot.cloud} />
          <WorkspaceRegistry workspaces={snapshot.workspaces} />
          <AgentCatalog packs={snapshot.packs} />
          <TeamConsole teams={snapshot.teams.items} packs={snapshot.packs} templates={snapshot.templates} workspaces={snapshot.workspaces} />
          {!activated ? <ActivationPath projection={observed} work={snapshot.work} workspaces={snapshot.workspaces} /> : null}
          <WorkConsole work={snapshot.work} workspaces={snapshot.workspaces} projection={observed} ownerEpoch={snapshot.hostEpoch} />
          <CloudPanel status={snapshot.cloud} />
          {observed ? <Dashboard projection={observed} /> : <section className="notice"><h2>Providers have not been observed</h2><p>Refresh providers to check installed CLIs and enable proven controls.</p></section>}
        </fieldset>
        <footer className="snapshot">Patchfleet {snapshot.appVersion} · Rust local host</footer>
      </>}
    </main>
  </HostContext.Provider>;
}

function TeamConsole({ teams, packs, templates, workspaces }: { teams: AgentTeam[]; packs: AgentPack[]; templates: TeamTemplate[]; workspaces: WorkspaceProjection }) {
  const orchestrators = packs.filter((pack) => pack.role === "orchestrator");
  const workers = packs.filter((pack) => pack.role !== "orchestrator");
  return (
    <section className="panel" aria-labelledby="teams-title">
      <div className="section-heading">
        <div><p className="eyebrow">Multi-agent delivery</p><h2 id="teams-title">Agent teams</h2></div>
        <span className="count">{teams.length} teams</span>
      </div>
      <div className="work-grid">
        <div className="work-stack">
          {teams.length === 0 ? <div className="empty-state"><strong>No team yet</strong><p>Choose ready agents and one bounded template.</p></div> : (
            <ul className="work-items">
              {teams.map((team) => (
                <li key={team.teamId}>
                  <div className="work-summary">
                    <div><h3>{team.name}</h3><p>{team.goal}</p><small>{team.templateId.replaceAll("-", " ")} · concurrency {team.settings.concurrency} · retry {team.settings.retryLimit}</small></div>
                    <span className={`badge ${team.status}`}>{team.status}</span>
                  </div>
                  {team.runtimeIssue ? <p className="note">This team’s workspace is no longer registered. Register a workspace and create a new team. Cancel this team if it is active.</p> : null}
                  <ol className="activation-steps">
                    {team.tasks.map((task) => {
                      const agent = team.agents.find((candidate) => candidate.agentId === task.agentId);
                      return (
                        <li key={task.taskId} className={task.status === "completed" ? "done" : ["running", "waiting_question", "waiting_approval"].includes(task.status) ? "active" : ""}>
                          <span>{task.attempt}</span>
                          <div>
                            <strong>{task.title}</strong>
                            <small>{agent?.pack.name} · {task.status} · attempt {task.attempt}/{task.maxAttempts}</small>
                            {task.status === "waiting_question" && task.question ? (
                              <HostForm className="work-form" namespace="team">
                                <input type="hidden" name="action" value="answer" /><input type="hidden" name="teamId" value={team.teamId} /><input type="hidden" name="expectedTeamRevision" value={team.revision} /><input type="hidden" name="questionId" value={task.question.questionId} />
                                <label>{task.question.prompt}<textarea name="answer" required maxLength={task.question.maxAnswerLength} rows={3} /></label><button type="submit">Answer</button>
                              </HostForm>
                            ) : null}
                            {task.status === "waiting_approval" ? (
                              <HostForm className="work-form" namespace="team">
                                <input type="hidden" name="teamId" value={team.teamId} /><input type="hidden" name="expectedTeamRevision" value={team.revision} /><input type="hidden" name="taskId" value={task.taskId} />
                                <label>Decision note <input name="note" maxLength={1000} /></label>
                                <div className="work-actions"><button name="action" value="approve" type="submit">Approve</button><button className="danger" name="action" value="reject" type="submit">Reject</button></div>
                              </HostForm>
                            ) : null}
                          </div>
                        </li>
                      );
                    })}
                  </ol>
                  <div className="work-actions">
                    {team.status === "draft" ? <TeamAction revision={team.revision} teamId={team.teamId} action="start">Start team</TeamAction> : null}
                    {["active", "waiting"].includes(team.status) ? <><TeamAction revision={team.revision} teamId={team.teamId} action="advance">Run ready work</TeamAction><TeamAction revision={team.revision} teamId={team.teamId} action="cancel" danger>Cancel team</TeamAction></> : null}
                  </div>
                  <details className="connection-settings"><summary>Agents and controls</summary>
                    <ul className="receipts">{team.agents.map((agent) => <li key={agent.agentId}><span className={`badge ${agent.status}`}>{agent.status}</span><div><strong>{agent.pack.name}</strong><small>{agent.pack.role}</small>{agent.agentId !== team.orchestratorAgentId && !["completed", "cancelled"].includes(agent.status) && ["active", "waiting"].includes(team.status) ? <HostForm namespace="team"><input type="hidden" name="action" value="cancel_agent" /><input type="hidden" name="teamId" value={team.teamId} /><input type="hidden" name="expectedTeamRevision" value={team.revision} /><input type="hidden" name="agentId" value={agent.agentId} /><button className="secondary" type="submit">Cancel agent</button></HostForm> : null}</div></li>)}</ul>
                  </details>
                </li>
              ))}
            </ul>
          )}
        </div>
        <div className="composer">
          <div className="section-heading"><div><p className="eyebrow">Team builder</p><h3>Form a local team</h3></div><span className="privacy-note">Goals stay local</span></div>
          <HostForm className="work-form" namespace="team">
            <input type="hidden" name="action" value="create" />
            <label>Team name<input name="name" required maxLength={80} placeholder="e.g. Billing launch" /></label>
            <label>Local project<select name="workspaceId" required defaultValue=""><option value="" disabled>Choose a registered project</option>{workspaces.items.map((workspace) => <option key={workspace.workspaceId} value={workspace.workspaceId}>{workspace.displayName}</option>)}</select></label>
            <label>Template<select name="templateId" defaultValue="product-feature">{templates.map((template) => <option key={template.id} value={template.id}>{template.name}</option>)}</select></label>
            <label>Orchestrator<select name="orchestratorPackId">{orchestrators.map((pack) => <option key={pack.id} value={pack.id}>{pack.name}</option>)}</select></label>
            {[1, 2, 3, 4].map((slot) => <label key={slot}>Agent {slot}{slot === 1 ? " (required)" : ""}<select name={`workerPack${slot}`} required={slot === 1} defaultValue=""><option value="">{slot === 1 ? "Choose an agent" : "No agent"}</option>{workers.map((pack) => <option key={pack.id} value={pack.id}>{pack.name} · {pack.role}</option>)}</select></label>)}
            <div className="work-actions"><label>Concurrency<select name="concurrency" defaultValue="2"><option>1</option><option>2</option><option>3</option><option>4</option></select></label><label>Retries<select name="retryLimit" defaultValue="1"><option>0</option><option>1</option><option>2</option><option>3</option></select></label></div>
            <label>Time budget<select name="timeoutMinutes" defaultValue="120"><option value="30">30 minutes</option><option value="60">1 hour</option><option value="120">2 hours</option><option value="240">4 hours</option><option value="480">8 hours</option></select></label>
            <label>Failure policy<select name="failurePolicy" defaultValue="stop"><option value="stop">Stop dependent work</option><option value="continue">Continue independent work</option></select></label>
            <label>Goal<textarea name="goal" required maxLength={10000} rows={5} placeholder="Describe the outcome and acceptance criteria." /></label>
            <button type="submit">Create team plan</button>
          </HostForm>
        </div>
      </div>
    </section>
  );
}

function TeamAction({ teamId, revision, action, danger = false, children }: { teamId: string; revision: number; action: "start" | "advance" | "cancel"; danger?: boolean; children: React.ReactNode }) {
  return <HostForm namespace="team"><input type="hidden" name="action" value={action} /><input type="hidden" name="teamId" value={teamId} /><input type="hidden" name="expectedTeamRevision" value={revision} /><button className={danger ? "danger" : undefined} type="submit">{children}</button></HostForm>;
}

function AgentCatalog({ packs }: { packs: AgentPack[] }) {
  const builtIn = packs.filter((pack) => pack.provenance.kind === "built-in").length;
  return (
    <details className="diagnostics-panel agent-catalog">
      <summary>
        <span><strong>Ready agent catalog</strong><small>Inspect roles, permissions, limits, and quality checks before forming a team</small></span>
        <span className="count">{builtIn} built-in · {packs.length - builtIn} custom</span>
      </summary>
      <div className="provider-grid">
        {packs.map((pack) => {
          const titleId = `${pack.id.replace(":", "-")}-title`;
          return (
            <article className="panel provider" key={pack.id} aria-labelledby={titleId}>
              <div className="section-heading">
                <div><p className="eyebrow">{pack.role}</p><h2 id={titleId}>{pack.name}</h2></div>
                <span className={`badge ${pack.provenance.kind === "built-in" ? "available" : "queued"}`}>{pack.provenance.kind}</span>
              </div>
              <p>{pack.description}</p>
              <dl>
                <div><dt>Provider</dt><dd>Codex</dd></div>
                <div><dt>Limit</dt><dd>{pack.limits.timeoutMinutes} min · {pack.limits.maxAttempts} attempts</dd></div>
                <div><dt>Permissions</dt><dd>{pack.permissions.join(", ")}</dd></div>
                <div><dt>Checks</dt><dd>{pack.qualityChecks.join(", ") || "Task-defined"}</dd></div>
              </dl>
              <details className="connection-settings">
                <summary>Inspect pack contract</summary>
                <p>{pack.instructions}</p>
                <p><strong>Expected output:</strong> {pack.expectedOutput}</p>
                <p><code>{pack.id}@{pack.version}</code> · {pack.provenance.source}</p>
              </details>
              {pack.provenance.kind === "local" ? <HostForm namespace="pack"><input type="hidden" name="action" value="remove" /><input type="hidden" name="packId" value={pack.id} /><button className="secondary" type="submit">Remove custom pack</button></HostForm> : null}
            </article>
          );
        })}
      </div>
      <HostForm namespace="pack" className="work-form"><input type="hidden" name="action" value="install" /><label>Custom pack JSON<textarea name="manifest" required maxLength={65536} rows={6} placeholder="Paste an inspectable agent pack manifest" /></label><button type="submit">Install custom pack</button></HostForm>
      <p className="snapshot">Custom packs are strict local JSON data. Import a manifest below; packs cannot load executable code or widen the Codex sandbox.</p>
    </details>
  );
}

function ReadinessOverview({ projection, cloud }: {
  projection: Projection | null;
  cloud: CloudStatus;
}) {
  const codex = projection?.observations.find((item) => item.provider.id === "codex");
  const codexReady = projection?.observations.some(supportsCodexControl) ?? false;
  const cloudReady = cloud.paired && !cloud.lastErrorCode;
  return (
    <section className="readiness" aria-labelledby="readiness-title">
      <div className="readiness-heading">
        <div>
          <p className="eyebrow">System readiness</p>
          <h2 id="readiness-title">Your control room</h2>
        </div>
        <p>Live work stays local. Cloud is an optional sanitized remote view.</p>
      </div>
      <div className="readiness-grid">
        <div className="readiness-card ready">
          <span className="status-dot" aria-hidden="true" />
          <div><span>Local engine</span><strong>Ready</strong></div>
          <small>Durable and restart-safe</small>
        </div>
        <div className={`readiness-card ${codexReady ? "ready" : "waiting"}`}>
          <span className="status-dot" aria-hidden="true" />
          <div><span>Codex control</span><strong>{codexReady ? "Ready" : projection ? "Observe only" : "Needs refresh"}</strong></div>
          <small>{codex?.provider.version ? `Version ${codex.provider.version}` : "No supported control state yet"}</small>
        </div>
        <div className={`readiness-card ${cloudReady ? "ready" : "waiting"}`}>
          <span className="status-dot" aria-hidden="true" />
          <div><span>Patchfleet Cloud</span><strong>{cloudReady ? "Connected" : cloud.paired ? "Retrying" : "Optional"}</strong></div>
          <small>{cloud.paired ? "Sanitized outbound sync" : "Local product stays complete"}</small>
        </div>
      </div>
    </section>
  );
}

function ActivationPath({ projection, work, workspaces }: {
  projection: Projection | null;
  work: WorkProjection;
  workspaces: WorkspaceProjection;
}) {
  const hasWork = work.items.length > 0;
  const hasStarted = work.items.some((item) => item.status !== "queued") || work.receipts.some((receipt) => receipt.reasonCode === "WORK_STARTED");
  return (
    <section className="activation" aria-labelledby="activation-title">
      <div>
        <p className="eyebrow">First run</p>
        <h2 id="activation-title">Three steps to controlled work</h2>
      </div>
      <ol className="activation-steps">
        <li className={projection ? "done" : "active"}><span>1</span><div><strong>Refresh providers</strong><small>Confirm what Patchfleet can observe and control.</small></div></li>
        <li className={hasWork ? "done" : projection ? "active" : ""}><span>2</span><div><strong>Queue a task</strong><small>{workspaces.items.length ? "Choose a registered project and give one clear instruction." : "Register a project once, then give one clear instruction."}</small></div></li>
        <li className={hasStarted ? "done" : hasWork ? "active" : ""}><span>3</span><div><strong>Start and monitor</strong><small>Codex runs locally; every action ends with a receipt.</small></div></li>
      </ol>
    </section>
  );
}

function CloudPanel({ status }: { status: CloudStatus }) {
  const errorMessage = status.paired && status.lastErrorCode
    ? ({
      CLOUD_UNAVAILABLE: "Cloud cannot be reached right now. Local work continues and sync will retry automatically.",
      CLOUD_AUTH_REJECTED: "This host no longer has Cloud access. Disconnect it here, then pair it again from Cloud.",
      CLOUD_CONFLICT: "Cloud has a newer or different snapshot. Upgrade recovery is automatic; genuinely older local state stays blocked.",
      CLOUD_PROTOCOL_INVALID: "Cloud returned an incompatible response. Local work remains safe while sync retries.",
    } as Record<string, string>)[status.lastErrorCode] ?? "Cloud sync could not finish. Local work remains available and Patchfleet will retry."
    : null;
  return (
    <section className="panel cloud-panel" aria-labelledby="cloud-title">
      <div className="section-heading">
        <div>
          <p className="eyebrow">Optional remote view</p>
          <h2 id="cloud-title">Patchfleet Cloud</h2>
        </div>
        <span className={`badge ${status.paired && !status.lastErrorCode ? "available" : "unavailable"}`}>
          {status.paired ? (status.lastErrorCode ? "Sync unavailable" : "Paired") : "Local only"}
        </span>
      </div>
      {status.paired ? (
        <>
          <dl>
            <div><dt>Cloud</dt><dd>{status.cloudUrl}</dd></div>
            <div><dt>Last sync</dt><dd>{status.lastSuccessAt ? time(status.lastSuccessAt) : "Waiting"}</dd></div>
            <div><dt>Privacy</dt><dd>Sanitized status only</dd></div>
          </dl>
          {errorMessage ? <p className="safe-error" role="status">{errorMessage}<code>{status.lastErrorCode}</code></p> : null}
          <details className="connection-settings">
            <summary>Connection settings</summary>
            <p>Host <code>{shortId(status.hostId)}</code></p>
            <HostForm namespace="cloud"><input type="hidden" name="action" value="sync" /><button className="secondary" type="submit">Sync now</button></HostForm>
            <HostForm namespace="cloud">
              <input type="hidden" name="action" value="disconnect" />
              <button className="secondary" type="submit">Disconnect Cloud</button>
            </HostForm>
          </details>
        </>
      ) : (
        <>
          {status.error ? <p className="safe-error" role="status">Cloud settings could not be read. Local work remains available.</p> : null}
          <HostForm className="work-form" namespace="cloud">
            <input type="hidden" name="action" value="pair" />
            <label>
              Cloud URL
              <input name="cloudUrl" type="url" required maxLength={2048} placeholder="https://cloud.example.com" autoComplete="url" />
            </label>
            <label>
              Host name
              <input name="displayName" required minLength={1} maxLength={80} autoComplete="off" />
            </label>
            <label>
              Pairing code
              <input name="pairingCode" required minLength={1} maxLength={256} autoComplete="off" />
            </label>
            <button type="submit">Pair this host</button>
          </HostForm>
          <p className="snapshot">Only operational IDs, states, revisions, capabilities, and coarse timestamps leave this host.</p>
        </>
      )}
    </section>
  );
}

function WorkConsole({ work, workspaces, projection, ownerEpoch }: {
  work: WorkProjection;
  workspaces: WorkspaceProjection;
  projection: Projection | null;
  ownerEpoch: string;
}) {
  const codexAvailable = projection?.observations.some(supportsCodexControl) ?? false;
  const latestReceipts = work.receipts.slice(-5).reverse();
  return (
    <div className="work-grid">
      <div className="work-stack">
        <section className="panel" aria-labelledby="work-items-title">
          <div className="section-heading">
            <div>
              <p className="eyebrow">Now</p>
              <h2 id="work-items-title">Your work</h2>
            </div>
            <span className={`badge ${codexAvailable ? "available" : "unavailable"}`}>
              Codex {codexAvailable ? "ready" : "control unavailable"}
            </span>
          </div>
          {work.items.length === 0 ? (
            <div className="empty-state">
              <strong>Nothing needs attention</strong>
              <p>Create a task and it will appear here before Codex starts.</p>
            </div>
          ) : (
            <ul className="work-items">
              {work.items.map((item) => {
                const itemRuns = work.runs.filter((candidate) => candidate.workItemId === item.workItemId);
                const run = itemRuns[itemRuns.length - 1];
                const workspace = workspaces.items.find(
                  (candidate) => candidate.workingDirectory === item.workingDirectory,
                );
                const staleRun = run?.status === "running" && run.ownerEpoch !== ownerEpoch;
                return (
                  <li key={item.workItemId}>
                    <div className="work-summary">
                      <div>
                        <h3>{item.title}</h3>
                        <p>{workspace ? <>Project <strong>{workspace.displayName}</strong></> : "Unregistered local project"}</p>
                        <details className="workspace-diagnostics">
                          <summary>Local path</summary>
                          <code>{item.workingDirectory}</code>
                        </details>
                        {run ? <p>Run <code>{shortId(run.runId)}</code> · turn <code>{shortId(run.providerTurnId)}</code></p> : null}
                        {staleRun ? <p>Control owner changed. Refresh providers to reconcile this run safely.</p> : null}
                      </div>
                      <span className={`badge ${item.status}`}>{item.status}</span>
                    </div>
                    <div className="work-actions">
                      {item.status === "queued" && codexAvailable ? (
                        <WorkAction action="start" item={item}>Start Codex</WorkAction>
                      ) : null}
                      {["failed", "interrupted", "blocked"].includes(item.status) && codexAvailable ? <WorkAction action="retry" item={item}>Queue retry</WorkAction> : null}
                      {item.status === "queued" ? (
                        <WorkAction action="remove" item={item}>Remove</WorkAction>
                      ) : null}
                      {run?.status === "running" && !staleRun && codexAvailable ? (
                        <RunCancel run={run} />
                      ) : null}
                    </div>
                  </li>
                );
              })}
            </ul>
          )}
        </section>

        <section className="panel" aria-labelledby="receipts-title">
          <div className="section-heading">
            <div>
              <p className="eyebrow">Proven outcomes</p>
              <h2 id="receipts-title">Recent activity</h2>
            </div>
            <span className="count">{latestReceipts.length} recent</span>
          </div>
          {latestReceipts.length === 0 ? (
            <div className="empty-state compact">
              <strong>No receipts yet</strong>
              <p>Every accepted command will leave a durable outcome here.</p>
            </div>
          ) : (
            <ul className="receipts">
              {latestReceipts.map((receipt) => (
                <li key={receipt.intentId}>
                  <span className={`badge ${receipt.outcome}`}>{receipt.outcome}</span>
                  <div>
                    <strong>{receiptTitle(receipt.reasonCode)}</strong>
                    <time dateTime={receipt.completedAt}>{time(receipt.completedAt)}</time>
                    <details className="receipt-diagnostics"><summary>Details</summary><code>{receipt.reasonCode}</code></details>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>

      <section id="queue-work" className="panel composer" aria-labelledby="create-work-title">
        <div className="section-heading">
          <div>
            <p className="eyebrow">Next task</p>
            <h2 id="create-work-title">Give Codex work</h2>
          </div>
          <span className="privacy-note">Stays local</span>
        </div>
        <HostForm className="work-form" namespace="work">
          <input type="hidden" name="action" value="enqueue" />
          <label>
            Task name
            <input name="title" required maxLength={160} autoComplete="off" placeholder="e.g. Verify the release build" />
          </label>
          <label>
            Project
            <select name="workspaceId" defaultValue="" aria-describedby="workspace-select-help">
              <option value="">{workspaces.items.length ? "Choose a project" : "No registered projects"}</option>
              {workspaces.items.map((workspace) => (
                <option key={workspace.workspaceId} value={workspace.workspaceId}>{workspace.displayName}</option>
              ))}
            </select>
            <span id="workspace-select-help">
              {workspaces.items.length
                ? "The selected path is resolved only on this machine."
                : <>Register a Git repository in Projects above.</>}
            </span>
          </label>
          <details className="advanced-path">
            <summary>Use another Git worktree once</summary>
            <label>
              Absolute Git worktree root
              <input
                name="workingDirectory"
                maxLength={4096}
                autoComplete="off"
                placeholder="/absolute/path/to/git-worktree"
                aria-describedby="worktree-path-help"
              />
              <span id="worktree-path-help">Run <code>pwd</code> inside the repository and paste the absolute path; relative paths and <code>~</code> are not accepted. Leave this blank when selecting a project above.</span>
            </label>
          </details>
          <label>
            What should Codex do?
            <textarea name="instruction" required maxLength={50000} rows={5} placeholder="Describe one bounded outcome." />
          </label>
          <button type="submit">Add to queue</button>
        </HostForm>
        <p className="snapshot">Review the queued task in Your work, then start it when ready.</p>
      </section>
    </div>
  );
}

function receiptTitle(reasonCode: string) {
  return ({
    WORK_ENQUEUED: "Task added to the queue",
    WORK_REMOVED: "Queued task removed",
    WORK_STARTED: "Codex started working",
    RUN_CANCELLED: "Run cancelled safely",
    COMMAND_EXPIRED: "Action expired without a change",
    PROVIDER_CONTROL_FAILED: "Codex control failed safely",
    RUN_SESSION_LOST: "Run control session was lost",
  } as Record<string, string>)[reasonCode] ?? "Action recorded";
}

function WorkAction({ action, item, children }: {
  action: "start" | "retry" | "remove";
  item: WorkItem;
  children: React.ReactNode;
}) {
  return (
    <HostForm namespace="work">
      <input type="hidden" name="action" value={action} />
      <input type="hidden" name="workItemId" value={item.workItemId} />
      <input type="hidden" name="expectedItemRevision" value={item.revision} />
      <button className={action === "remove" ? "secondary" : undefined} type="submit">{children}</button>
    </HostForm>
  );
}

function RunCancel({ run }: { run: Run }) {
  return (
    <HostForm namespace="work">
      <input type="hidden" name="action" value="cancel" />
      <input type="hidden" name="runId" value={run.runId} />
      <input type="hidden" name="expectedRunRevision" value={run.revision} />
      <button className="danger" type="submit">Cancel run</button>
    </HostForm>
  );
}

function Dashboard({ projection }: { projection: Projection }) {
  return (
    <details className="diagnostics-panel">
      <summary>
        <span><strong>Provider diagnostics</strong><small>Versions, capabilities, and recent sessions</small></span>
        <span className="count">{projection.observations.length} providers</span>
      </summary>
      <div className="provider-grid">
        {projection.observations.map((observation) => (
          <ProviderObservation key={observation.provider.id} observation={observation} />
        ))}
      </div>
    </details>
  );
}

function ProviderObservation({ observation }: { observation: Observation }) {
  const { provider, sessions } = observation;
  const providerTitle = `${provider.id}-provider-title`;
  const sessionsTitle = `${provider.id}-sessions-title`;
  return (
    <article className="provider-group">
      <section className="provider panel" aria-labelledby={providerTitle}>
        <div className="section-heading">
          <div>
            <p className="eyebrow">Provider</p>
            <h2 id={providerTitle}>{provider.displayName}</h2>
          </div>
          <span className={`badge ${provider.state}`}>{providerLabel(provider.state)}</span>
        </div>
        <dl>
          <div>
            <dt>CLI version</dt>
            <dd>{provider.version ?? "Not available"}</dd>
          </div>
          <div>
            <dt>Recent observation</dt>
            <dd>{provider.capabilities.recentObservation ? "Supported" : "Unavailable"}</dd>
          </div>
          <div>
            <dt>Explicit live status</dt>
            <dd>{provider.capabilities.explicitLiveStatus ? "Supported" : "Unavailable"}</dd>
          </div>
        </dl>
        {provider.error ? (
          <p className="safe-error" role="status">
            {provider.error.message} <code>{provider.error.code}</code>
          </p>
        ) : null}
        <p className="snapshot">
          Stored local snapshot from <time dateTime={observation.observedAt}>{time(observation.observedAt)}</time>.
          It is not a live feed and remains available after restart.
        </p>
      </section>

      <section className="panel sessions-panel" aria-labelledby={sessionsTitle}>
        <div className="section-heading">
          <div>
            <p className="eyebrow">Most recent</p>
            <h2 id={sessionsTitle}>{provider.displayName} sessions</h2>
          </div>
          <span className="count">{sessions.length} / 20</span>
        </div>
        {sessions.length === 0 ? (
          <p className="empty">No recent interactive sessions were returned.</p>
        ) : (
          <ul className="sessions">
            {sessions.map((session) => (
              <li key={session.providerSessionId}>
                <div>
                  <code>{shortId(session.providerSessionId)}</code>
                  <p>
                    {session.createdAt ? (
                      <>Created <time dateTime={session.createdAt}>{time(session.createdAt)}</time></>
                    ) : (
                      "Creation time not supplied"
                    )}
                  </p>
                </div>
                <div className="session-state">
                  <span className={`badge ${session.status}`}>{session.status}</span>
                  <small>
                    {session.status === "unknown" ? "Live status not observed" : `Observed ${time(session.lastObservedAt)}`}
                  </small>
                </div>
              </li>
            ))}
          </ul>
        )}
      </section>
    </article>
  );
}


type TeamTemplate = { id: string; name: string; description: string };
type HostSnapshot = { appVersion: string; hostEpoch: string; observation: Projection | null; work: WorkProjection; workspaces: WorkspaceProjection; packs: AgentPack[]; templates: TeamTemplate[]; teams: AgentTeamProjection; cloud: CloudStatus };

function supportsCodexControl(observation: Observation) {
  const p = observation.provider;
  const match = /^(\d+)\.(\d+)\.(\d+)(?:\+[A-Za-z0-9.-]+)?$/.exec(p.version ?? "");
  if (p.id !== "codex" || p.state !== "available" || !p.capabilities.recentObservation || !p.capabilities.explicitLiveStatus || !match) return false;
  const version = match.slice(1).map(Number);
  const minimum = [0, 144, 1];
  if (version.some((part) => !Number.isSafeInteger(part))) return false;
  for (let i = 0; i < version.length; i++) if (version[i] !== minimum[i]) return version[i] > minimum[i];
  return true;
}

const HostContext = createContext<{busy: boolean; dispatch: (command: HostCommand) => Promise<boolean>; report: (message: string) => void} | null>(null);

function HostForm({namespace, className, children}: {namespace: FormNamespace; className?: string; children: ReactNode}) {
  const host = useContext(HostContext)!;
  const clickedSubmitter = useRef<HTMLButtonElement | null>(null);
  const onSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (host.busy) return;
    const form = event.currentTarget;
    const submitter = ((event.nativeEvent as SubmitEvent).submitter as HTMLButtonElement | null)
      ?? clickedSubmitter.current ?? form.querySelector<HTMLButtonElement>("button[type='submit']");
    clickedSubmitter.current = null;
    const data = new FormData(form);
    if (submitter?.name) data.set(submitter.name, submitter.value);
    try {
      const command = commandFromForm(namespace, Object.fromEntries(data.entries()));
      if (await host.dispatch(command) && ["work_enqueue", "team_create", "workspace_register", "pack_install", "cloud_pair", "team_answer"].includes(command.action)) form.reset();
    } catch (error) { host.report(error instanceof Error ? error.message : "Invalid form values"); }
  };
  return <form className={className} onSubmit={onSubmit} onClickCapture={(event) => {
    clickedSubmitter.current = event.target instanceof Element ? event.target.closest<HTMLButtonElement>("button[type='submit']") : null;
  }}>{children}</form>;
}

function WorkspaceRegistry({workspaces}: {workspaces: WorkspaceProjection}) {
  return <section className="panel" aria-labelledby="projects-title"><div className="section-heading"><div><p className="eyebrow">Local worktrees</p><h2 id="projects-title">Projects</h2></div><span className="count">{workspaces.items.length} registered</span></div>
    <div className="work-grid"><div>{workspaces.items.length ? <ul className="work-items">{workspaces.items.map((workspace) => <li key={workspace.workspaceId}><h3>{workspace.displayName}</h3><code>{workspace.workingDirectory}</code><HostForm namespace="workspace"><input type="hidden" name="action" value="remove" /><input type="hidden" name="workspaceId" value={workspace.workspaceId} /><input type="hidden" name="expectedWorkspaceRevision" value={workspace.revision} /><button className="secondary" type="submit">Unregister</button></HostForm></li>)}</ul> : <div className="empty-state"><strong>No registered project</strong><p>Add an existing Git worktree root to use it for tasks and teams.</p></div>}</div>
      <HostForm namespace="workspace" className="work-form"><input type="hidden" name="action" value="register" /><label>Project name<input name="displayName" maxLength={160} placeholder="Optional display name" /></label><label>Absolute Git worktree path<input name="workingDirectory" required maxLength={4096} autoComplete="off" placeholder="Absolute path to a Git worktree" /></label><button type="submit">Register project</button><p className="snapshot">Unregistering removes this registration. Repository files stay on disk.</p></HostForm>
    </div>
  </section>;
}
