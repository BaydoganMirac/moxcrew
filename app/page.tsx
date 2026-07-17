import { randomUUID } from "node:crypto";
import { publicCloudStatus, readCloudState } from "@/lib/cloud/connection.mjs";
import { supportsCodexControl } from "@/lib/providers/codex.mjs";
import { readProjection, readWorkProjection, readWorkspaceProjection } from "@/lib/runtime/observation-store.mjs";
import { currentWorkControlOwnerEpoch } from "@/lib/runtime/work-queue.mjs";

export const dynamic = "force-dynamic";

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

async function loadState() {
  try {
    const [projection, work, workspaces] = await Promise.all([
      readProjection(),
      readWorkProjection(),
      readWorkspaceProjection(),
    ]);
    const cloud = await readCloudState()
      .then(publicCloudStatus)
      .catch(() => ({ paired: false as const, error: true }));
    return {
      kind: "ready" as const,
      projection: projection as Projection | null,
      work: (work ?? { schemaVersion: 1, revision: 0, items: [], runs: [], receipts: [] }) as WorkProjection,
      workspaces: (workspaces ?? { schemaVersion: 1, revision: 0, items: [], receipts: [] }) as WorkspaceProjection,
      cloud: cloud as CloudStatus,
    };
  } catch {
    return { kind: "fatal" as const };
  }
}

export default async function Home({ searchParams }: {
  searchParams: Promise<{ work?: string | string[] }>;
}) {
  const workCode = (await searchParams).work;
  const result = await loadState();

  return (
    <main className="app-shell">
      <header className="app-header">
        <a className="brand" href="/" aria-label="Patchfleet home">
          <span className="brand-mark" aria-hidden="true">P</span>
          <span>Patchfleet</span>
          <span className="local-label">Local</span>
        </a>
        <span className="privacy-pill"><span aria-hidden="true">●</span> Local data stays local</span>
      </header>

      <section className="hero" aria-labelledby="page-title">
        <div className="hero-copy">
          <p className="eyebrow">AI agent command center</p>
          <h1 id="page-title">Keep every coding agent in view.</h1>
          <p className="summary">
            Queue real work, monitor honest provider state, and stop a risky run from one private control room.
          </p>
          <div className="hero-actions">
            <form action="/api/observe" method="post">
              <button type="submit">Refresh providers</button>
            </form>
            <a className="button-link secondary" href="#queue-work">Queue a task</a>
          </div>
        </div>
        <div className="trust-card">
          <span className="trust-icon" aria-hidden="true">✓</span>
          <div>
            <strong>Your machine stays in charge</strong>
            <p>Source, prompts, paths, credentials, and provider output are not sent to Patchfleet Cloud.</p>
          </div>
        </div>
      </section>

      {result.kind === "fatal" ? (
        <section className="notice error panel" aria-labelledby="storage-error" role="alert">
          <h2 id="storage-error">Local storage needs attention</h2>
          <p>The durable local projection is corrupt. No provider process was started.</p>
        </section>
      ) : (
        <>
          <WorkFeedback code={typeof workCode === "string" ? workCode : null} />
          <ReadinessOverview projection={result.projection} cloud={result.cloud} />
          <ActivationPath projection={result.projection} work={result.work} workspaces={result.workspaces} />
          <WorkConsole
            work={result.work}
            workspaces={result.workspaces}
            projection={result.projection}
            ownerEpoch={currentWorkControlOwnerEpoch()}
          />
          <CloudPanel status={result.cloud} />
          {result.projection === null ? (
            <section className="notice" aria-labelledby="never-observed">
              <h2 id="never-observed">Providers have not been observed</h2>
              <p>Refresh once to check the installed CLIs and enable proven controls.</p>
            </section>
          ) : (
            <Dashboard projection={result.projection} />
          )}
        </>
      )}
    </main>
  );
}

function WorkFeedback({ code }: { code: string | null }) {
  const feedback = code && Object.hasOwn(WORK_FEEDBACK, code) ? WORK_FEEDBACK[code] : null;
  if (!feedback) return null;
  return (
    <section className={`feedback ${feedback.tone}`} role={feedback.tone === "error" ? "alert" : "status"} data-testid="work-feedback">
      <span className="feedback-mark" aria-hidden="true">{feedback.tone === "success" ? "✓" : "!"}</span>
      <div>
        <h2>{feedback.title}</h2>
        <p>{feedback.detail}</p>
      </div>
      <code>{code}</code>
    </section>
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
            <div><dt>Host</dt><dd><code>{shortId(status.hostId)}</code></dd></div>
            <div><dt>Last sync</dt><dd>{status.lastSuccessAt ? time(status.lastSuccessAt) : "Waiting"}</dd></div>
          </dl>
          {status.lastErrorCode ? <p className="safe-error" role="status">Sync will retry. <code>{status.lastErrorCode}</code></p> : null}
          <form action="/api/cloud" method="post">
            <input type="hidden" name="action" value="disconnect" />
            <button className="secondary" type="submit">Disconnect Cloud</button>
          </form>
        </>
      ) : (
        <>
          {status.error ? <p className="safe-error" role="status">Cloud settings could not be read. Local work remains available.</p> : null}
          <form className="work-form" action="/api/cloud" method="post">
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
          </form>
          <p className="snapshot">Only operational IDs, states, revisions, capabilities, and coarse timestamps leave this host.</p>
        </>
      )}
    </section>
  );
}

function commandId() {
  return `cmd:${randomUUID()}`;
}

function CommandFields() {
  const createdAt = new Date();
  return (
    <>
      <input type="hidden" name="commandId" value={commandId()} />
      <input type="hidden" name="createdAt" value={createdAt.toISOString()} />
      <input type="hidden" name="expiresAt" value={new Date(createdAt.valueOf() + 5 * 60_000).toISOString()} />
    </>
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
      <section id="queue-work" className="panel composer" aria-labelledby="create-work-title">
        <div className="section-heading">
          <div>
            <p className="eyebrow">Local work intake</p>
            <h2 id="create-work-title">Queue Codex work</h2>
          </div>
          <span className="count">{work.items.length} items</span>
        </div>
        <form className="work-form" action="/api/work" method="post">
          <input type="hidden" name="action" value="enqueue" />
          <CommandFields />
          <label>
            Title
            <input name="title" required maxLength={160} autoComplete="off" placeholder="e.g. Verify the release build" />
          </label>
          <label>
            Project
            <select name="workspaceId" defaultValue="" aria-describedby="workspace-select-help">
              <option value="">{workspaces.items.length ? "Select a registered project" : "No registered projects"}</option>
              {workspaces.items.map((workspace) => (
                <option key={workspace.workspaceId} value={workspace.workspaceId}>{workspace.displayName}</option>
              ))}
            </select>
            <span id="workspace-select-help">
              {workspaces.items.length
                ? "The selected path is resolved only on this machine."
                : <>Register the current Git repository with <code>patchfleet workspace add .</code>, then refresh this page.</>}
            </span>
          </label>
          <details className="advanced-path">
            <summary>Advanced: use an unregistered path once</summary>
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
            Instruction
            <textarea name="instruction" required maxLength={50000} rows={6} placeholder="Describe one bounded outcome. Patchfleet will run it locally through Codex." />
          </label>
          <button type="submit">Add task to queue</button>
        </form>
        <p className="snapshot">Add the item to the queue, then start it from Work items. Instructions and paths stay in the local work projection.</p>
      </section>

      <div className="work-stack">
        <section className="panel" aria-labelledby="work-items-title">
          <div className="section-heading">
            <div>
              <p className="eyebrow">Durable queue</p>
              <h2 id="work-items-title">Work items</h2>
            </div>
            <span className={`badge ${codexAvailable ? "available" : "unavailable"}`}>
              Codex {codexAvailable ? "ready" : "control unavailable"}
            </span>
          </div>
          {work.items.length === 0 ? (
            <div className="empty-state">
              <strong>No tasks in the queue</strong>
              <p>Your first task will appear here before anything starts.</p>
            </div>
          ) : (
            <ul className="work-items">
              {work.items.map((item) => {
                const run = work.runs.find((candidate) => candidate.workItemId === item.workItemId);
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
              <h2 id="receipts-title">Recent receipts</h2>
            </div>
            <span className="count">revision {work.revision}</span>
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
                  <code>{receipt.reasonCode}</code>
                  <time dateTime={receipt.completedAt}>{time(receipt.completedAt)}</time>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>
    </div>
  );
}

function WorkAction({ action, item, children }: {
  action: "start" | "remove";
  item: WorkItem;
  children: React.ReactNode;
}) {
  return (
    <form action="/api/work" method="post">
      <input type="hidden" name="action" value={action} />
      <CommandFields />
      <input type="hidden" name="workItemId" value={item.workItemId} />
      <input type="hidden" name="expectedItemRevision" value={item.revision} />
      <button className={action === "remove" ? "secondary" : undefined} type="submit">{children}</button>
    </form>
  );
}

function RunCancel({ run }: { run: Run }) {
  return (
    <form action="/api/work" method="post">
      <input type="hidden" name="action" value="cancel" />
      <CommandFields />
      <input type="hidden" name="runId" value={run.runId} />
      <input type="hidden" name="expectedRunRevision" value={run.revision} />
      <button className="danger" type="submit">Cancel run</button>
    </form>
  );
}

function Dashboard({ projection }: { projection: Projection }) {
  return (
    <div className="provider-grid">
      {projection.observations.map((observation) => (
        <ProviderObservation key={observation.provider.id} observation={observation} />
      ))}
    </div>
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
