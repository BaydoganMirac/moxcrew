import { randomUUID } from "node:crypto";
import { publicCloudStatus, readCloudState } from "@/lib/cloud/connection.mjs";
import { supportsCodexControl } from "@/lib/providers/codex.mjs";
import { readProjection, readWorkProjection } from "@/lib/runtime/observation-store.mjs";
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
    const [projection, work] = await Promise.all([readProjection(), readWorkProjection()]);
    const cloud = await readCloudState()
      .then(publicCloudStatus)
      .catch(() => ({ paired: false as const, error: true }));
    return {
      kind: "ready" as const,
      projection: projection as Projection | null,
      work: (work ?? { schemaVersion: 1, revision: 0, items: [], runs: [], receipts: [] }) as WorkProjection,
      cloud: cloud as CloudStatus,
    };
  } catch {
    return { kind: "fatal" as const };
  }
}

export default async function Home() {
  const result = await loadState();

  return (
    <main>
      <header className="hero">
        <div>
          <p className="eyebrow">Local-only console</p>
          <h1>Patchfleet</h1>
          <p className="summary">
            Restart-safe local work and honest lifecycle metadata from supported provider surfaces.
          </p>
        </div>
        <form action="/api/observe" method="post">
          <button type="submit">Refresh providers</button>
        </form>
      </header>

      {result.kind === "fatal" ? (
        <section className="notice error" aria-labelledby="storage-error" role="alert">
          <h2 id="storage-error">Local storage needs attention</h2>
          <p>The durable local projection is corrupt. No provider process was started.</p>
        </section>
      ) : (
        <>
          <WorkConsole
            work={result.work}
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

function CloudPanel({ status }: { status: CloudStatus }) {
  return (
    <section aria-labelledby="cloud-title">
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
            <button type="submit">Disconnect Cloud</button>
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

function WorkConsole({ work, projection, ownerEpoch }: {
  work: WorkProjection;
  projection: Projection | null;
  ownerEpoch: string;
}) {
  const codexAvailable = projection?.observations.some(supportsCodexControl) ?? false;
  const latestReceipts = work.receipts.slice(-5).reverse();
  return (
    <>
      <section aria-labelledby="create-work-title">
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
            <input name="title" required maxLength={160} autoComplete="off" />
          </label>
          <label>
            Git worktree root
            <input name="workingDirectory" required maxLength={4096} autoComplete="off" />
          </label>
          <label>
            Instruction
            <textarea name="instruction" required maxLength={50000} rows={5} />
          </label>
          <button type="submit">Add to queue</button>
        </form>
        <p className="snapshot">Instructions and paths stay in the local work projection.</p>
      </section>

      <section aria-labelledby="work-items-title">
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
          <p className="empty">No local work has been queued.</p>
        ) : (
          <ul className="work-items">
            {work.items.map((item) => {
              const run = work.runs.find((candidate) => candidate.workItemId === item.workItemId);
              const staleRun = run?.status === "running" && run.ownerEpoch !== ownerEpoch;
              return (
                <li key={item.workItemId}>
                  <div className="work-summary">
                    <div>
                      <h3>{item.title}</h3>
                      <p><code>{item.workingDirectory}</code></p>
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

      <section aria-labelledby="receipts-title">
        <div className="section-heading">
          <div>
            <p className="eyebrow">Safe outcomes</p>
            <h2 id="receipts-title">Recent receipts</h2>
          </div>
          <span className="count">revision {work.revision}</span>
        </div>
        {latestReceipts.length === 0 ? (
          <p className="empty">No local command receipts yet.</p>
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
    </>
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
      <button type="submit">{children}</button>
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
      <button type="submit">Cancel run</button>
    </form>
  );
}

function Dashboard({ projection }: { projection: Projection }) {
  return projection.observations.map((observation) => (
    <ProviderObservation key={observation.provider.id} observation={observation} />
  ));
}

function ProviderObservation({ observation }: { observation: Observation }) {
  const { provider, sessions } = observation;
  const providerTitle = `${provider.id}-provider-title`;
  const sessionsTitle = `${provider.id}-sessions-title`;
  return (
    <>
      <section className="provider" aria-labelledby={providerTitle}>
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

      <section aria-labelledby={sessionsTitle}>
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
    </>
  );
}
