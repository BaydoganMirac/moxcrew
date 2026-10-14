// Regenerate Rust migration fixtures from the shipping JS projectors/runtime.
// Only task-owned temporary state is used; no installed Patchfleet data is read.
import { createHash } from "node:crypto";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { projectWorkEvents } from "../lib/domain/work.mjs";
import { projectWorkspaceEvents } from "../lib/domain/workspace.mjs";
import { projectAgentPackEvents } from "../lib/domain/agent-pack.mjs";
import { projectAgentTeamEvents } from "../lib/domain/agent-team.mjs";
import { safeObservationError } from "../lib/domain/observation.mjs";
import { BUILT_IN_AGENT_PACKS } from "../lib/agents/built-in-packs.mjs";
import { persistObservation, projectEvents, replayEvents } from "../lib/runtime/observation-store.mjs";
import { applyWorkCommand, applyWorkControlCommand, reconcileWorkControlOwnership } from "../lib/runtime/work-queue.mjs";
import { installAgentPack, removeAgentPack } from "../lib/runtime/agent-pack-registry.mjs";
import { registerWorkspace, removeWorkspace, applyWorkspaceCommand } from "../lib/runtime/workspace-registry.mjs";
import { createAgentTeam, startAgentTeam, advanceAgentTeam, cancelAgentTeam, cancelTeamAgent, answerTeamQuestion, decideTeamApproval } from "../lib/runtime/team-orchestrator.mjs";

const root = await mkdtemp(join(tmpdir(), "patchfleet-parity-"));
const fixture = [];
const t = (seconds = 0) => new Date(Date.UTC(2026, 6, 20, 12, 0, seconds)).toISOString();
const expires = t(3600);
const uuid = (digit) => `${digit.repeat(8)}-${digit.repeat(4)}-4${digit.repeat(3)}-8${digit.repeat(3)}-${digit.repeat(12)}`;
function projected(events) { return {observation:projectEvents(events),work:projectWorkEvents(events),workspaces:projectWorkspaceEvents(events),packs:projectAgentPackEvents(events),teams:projectAgentTeamEvents(events)}; }
function canonical(value) {
  if (Array.isArray(value)) return value.map(canonical);
  if (value && typeof value === "object") return Object.fromEntries(Object.keys(value).sort().map(key=>[key,canonical(value[key])]));
  return value;
}
const digest = value => createHash("sha256").update(JSON.stringify(canonical(value))).digest("hex");
const sanitize = (events) => JSON.parse(JSON.stringify(events).replaceAll(root, "/fixture"));
async function capture(name, dataDir) {
  const events = sanitize(await replayEvents({dataDir}));
  // Keep full final projections for readable diffs, and exact canonical JSON
  // digests at each intermediate event without duplicating megabytes of packs.
  fixture.push({name,events,expected:projected(events),prefixSha256:events.map((_,i) => digest(projected(events.slice(0,i+1))))});
}
async function setup(name) {
  const dataDir = join(root,name,"data"); const workspace = join(root,name,"project");
  await mkdir(join(workspace,".git"),{recursive:true});
  const workspaceId = `workspace:${uuid("1")}`;
  await registerWorkspace(workspace,{dataDir,workspaceId,commandId:`cmd:${uuid("1")}`,now:()=>t()});
  return {dataDir,workspace,workspaceId};
}
function intent(type, payload, identity, createdAt = t()) {
  return {schemaVersion:1,intentId:`intent:${identity}`,idempotencyKey:`key:${identity}`,type,actorId:"local-owner",createdAt,expiresAt:expires,payload};
}
function enqueue(workspace, workItemId, identity = workItemId, at = t()) {
  return intent("enqueue_work",{workItem:{schemaVersion:1,workItemId,title:`Work ${workItemId}`,instruction:"Fixture-local instruction.",providerId:"codex",workingDirectory:workspace,status:"queued",createdAt:at,revision:1}},identity,at);
}
function control(status = "running") {
  return (command, options) => applyWorkControlCommand(command,{...options,
    prepare:async({intentId})=>({providerSessionId:`session:${intentId}`}),
    start:async({intentId,providerSessionId})=>({providerSessionId,providerTurnId:`turn:${intentId}`,status,terminalAt:status==="running"?null:options.now()}),
    cancel:async()=>undefined,
  });
}
async function observe(dataDir, providerId, sessions, observedAt=t(1), state="available", code) {
  const displayName = {codex:"Codex",claude:"Claude Code",gemini:"Gemini CLI"}[providerId];
  return persistObservation({provider:{id:providerId,displayName,state,version:"1.2.3-beta.1+fixture",capabilities:{recentObservation:state==="available",explicitLiveStatus:state==="available"},...(code?{error:safeObservationError(providerId,code)}:{})},observedAt,sessions},{dataDir});
}
async function complete(dataDir,status="completed",at=t(30)) {
  const work = projectWorkEvents(await replayEvents({dataDir}));
  const sessions = work.runs.filter(r=>r.status==="running").map(r=>({providerSessionId:r.providerSessionId,status,createdAt:r.startedAt,lastObservedAt:at,terminalAt:at}));
  await observe(dataDir,"codex",sessions,at);
}
try {
  const a = await setup("work");
  await observe(a.dataDir,"claude",[],t(1),"unavailable","CLAUDE_NOT_FOUND");
  await observe(a.dataDir,"gemini",[{providerSessionId:"gemini-shared-id",status:"running",createdAt:null,lastObservedAt:t(2)}],t(2));
  await applyWorkCommand(enqueue(a.workspace,"work-a"),{dataDir:a.dataDir,now:()=>t(3)});
  await applyWorkCommand(enqueue(a.workspace,"work-b","b",t(1)),{dataDir:a.dataDir,now:()=>t(4)});
  await applyWorkCommand(enqueue(a.workspace,"work-a","duplicate"),{dataDir:a.dataDir,now:()=>t(5)});
  await applyWorkCommand(intent("remove_queued_work",{workItemId:"work-b",expectedItemRevision:2},"stale"),{dataDir:a.dataDir,now:()=>t(5)});
  await applyWorkCommand(intent("remove_queued_work",{workItemId:"missing",expectedItemRevision:1},"missing"),{dataDir:a.dataDir,now:()=>t(5)});
  await applyWorkCommand(intent("remove_queued_work",{workItemId:"work-b",expectedItemRevision:1},"remove"),{dataDir:a.dataDir,now:()=>t(6)});
  await control()(intent("start_work",{workItemId:"work-a",expectedItemRevision:1},"start-a"),{dataDir:a.dataDir,now:()=>t(7),ownerEpoch:"fixture-epoch"});
  await applyWorkCommand(intent("remove_queued_work",{workItemId:"work-a",expectedItemRevision:1},"remove-running"),{dataDir:a.dataDir,now:()=>t(8)});
  await control()(intent("cancel_run",{runId:"run:intent:start-a",expectedRunRevision:1},"cancel-a"),{dataDir:a.dataDir,now:()=>t(9),ownerEpoch:"fixture-epoch"});
  await applyWorkCommand({...enqueue(a.workspace,"expired","expired"),expiresAt:t(1)},{dataDir:a.dataDir,now:()=>t(10)});
  await capture("work-control-receipts-and-observation",a.dataDir);

  const b = await setup("unknown");
  await applyWorkCommand(enqueue(b.workspace,"unknown-work"),{dataDir:b.dataDir,now:()=>t()});
  await applyWorkControlCommand(intent("start_work",{workItemId:"unknown-work",expectedItemRevision:1},"unknown"),{dataDir:b.dataDir,now:()=>t(1),ownerEpoch:"old-epoch",prepare:async()=>({providerSessionId:"unknown-session"}),start:async()=>{throw Object.assign(new Error("fixture"),{code:"CODEX_CONTROL_TIMEOUT"});}});
  await capture("ambiguous-start-blocks",b.dataDir);

  const c = await setup("recovery");
  await applyWorkCommand(enqueue(c.workspace,"lost-work"),{dataDir:c.dataDir,now:()=>t()});
  await control()(intent("start_work",{workItemId:"lost-work",expectedItemRevision:1},"lost"),{dataDir:c.dataDir,now:()=>t(1),ownerEpoch:"lost-epoch"});
  await reconcileWorkControlOwnership({dataDir:c.dataDir,now:()=>t(2),ownerEpoch:"new-epoch",isActive:()=>false});
  await capture("restart-session-loss",c.dataDir);

  const d = await setup("workspace");
  const duplicate = {schemaVersion:1,workspaceId:`workspace:${uuid("2")}`,displayName:"Duplicate",workingDirectory:d.workspace,createdAt:t(),revision:1};
  await applyWorkspaceCommand({schemaVersion:1,intentId:`cmd:${uuid("2")}`,idempotencyKey:`cmd:${uuid("2")}`,type:"register_workspace",actorId:"local-owner",createdAt:t(),expiresAt:expires,payload:{workspace:duplicate}},{dataDir:d.dataDir,now:()=>t(1)});
  await removeWorkspace(d.workspaceId,{dataDir:d.dataDir,commandId:`cmd:${uuid("3")}`,expectedWorkspaceRevision:2,now:()=>t(2)});
  await removeWorkspace(d.workspaceId,{dataDir:d.dataDir,commandId:`cmd:${uuid("4")}`,now:()=>t(3)});
  await removeWorkspace(d.workspaceId,{dataDir:d.dataDir,commandId:`cmd:${uuid("5")}`,now:()=>t(4)});
  await capture("workspace-receipts-and-removal",d.dataDir);

  const p = await setup("packs");
  const custom = {...BUILT_IN_AGENT_PACKS.find(x=>x.role==="qa"),id:"pack:fixture-qa",name:"Fixture QA",provenance:{kind:"local",source:"fixture.json"}};
  await installAgentPack(custom,{dataDir:p.dataDir,now:()=>t()});
  await installAgentPack({...custom,version:"1.1.0"},{dataDir:p.dataDir,now:()=>t(1)});
  await removeAgentPack(custom.id,{dataDir:p.dataDir,now:()=>t(2)});
  await installAgentPack(custom,{dataDir:p.dataDir,now:()=>t(3)});
  await capture("pack-update-removal-reinstall",p.dataDir);

  for (const scenario of ["product","release","retry","agent-cancel","timeout","continue"]) {
    const s = await setup(scenario); let clock = 0; const now = () => t(clock++);
    const templateId = scenario==="release"?"release":scenario==="product"||scenario==="continue"?"product-feature":"bug-fix";
    const workerPackIds = scenario==="release"?["pack:qa","pack:release"]:scenario==="product"?["pack:product","pack:backend","pack:frontend","pack:qa"]:scenario==="continue"?["pack:product","pack:research","pack:qa"]:["pack:fullstack","pack:qa"];
    const team = await createAgentTeam({name:`Fixture ${scenario}`,goal:"Fixture-local team goal.",workspaceId:s.workspaceId,templateId,workerPackIds,settings:{concurrency:scenario==="product"||scenario==="continue"?2:1,retryLimit:scenario==="retry"?1:0,timeoutMinutes:5,failurePolicy:scenario==="continue"?"continue":"stop"}},{dataDir:s.dataDir,now});
    let calls = 0;
    const applyControl = scenario==="continue"?(command,options)=>control(calls++===0?"failed":"running")(command,options):control(scenario==="retry"?"failed":"running");
    let state = await startAgentTeam(team.teamId,{dataDir:s.dataDir,now,providerAvailable:true,applyControl});
    if (scenario==="product") {
      for(let i=0;i<3;i++){await complete(s.dataDir,"completed",t(30+i));state=await advanceAgentTeam(team.teamId,{dataDir:s.dataDir,now,providerAvailable:true,applyControl:control()});}
    } else if(scenario==="release") {
      await complete(s.dataDir); state=await advanceAgentTeam(team.teamId,{dataDir:s.dataDir,now,providerAvailable:true,applyControl:control()});
      const release = state.tasks.find(task=>task.approvalRequired);
      await answerTeamQuestion(team.teamId,release.question.questionId,"Prepare local checks only.",{dataDir:s.dataDir,now});
      await decideTeamApproval(team.teamId,release.taskId,"approved","Fixture owner accepted.",{dataDir:s.dataDir,now});
      await advanceAgentTeam(team.teamId,{dataDir:s.dataDir,now,providerAvailable:true,applyControl:control()});
    } else if(scenario==="retry") {
      await advanceAgentTeam(team.teamId,{dataDir:s.dataDir,now,providerAvailable:true,applyControl:control()});
      await cancelAgentTeam(team.teamId,{dataDir:s.dataDir,now,providerAvailable:true,applyControl:control()});
    } else if(scenario==="agent-cancel") {
      const agent = state.agents.find(agent=>agent.status==="running");
      await cancelTeamAgent(team.teamId,agent.agentId,{dataDir:s.dataDir,now,providerAvailable:true,applyControl:control()});
    } else if(scenario==="timeout") {
      await advanceAgentTeam(team.teamId,{dataDir:s.dataDir,now:()=>t(400),providerAvailable:true,applyControl:control()});
    }
    await capture(`team-${scenario}`,s.dataDir);
  }
  await writeFile(fileURLToPath(new URL("../crates/patchfleet-core/tests/fixtures/parity-v1.json",import.meta.url)),JSON.stringify({schemaVersion:1,generator:"tests/parity-generate.mjs",cases:fixture},null,2)+"\n");
  console.log(`Generated ${fixture.length} cases, ${fixture.reduce((n,c)=>n+c.events.length,0)} events and replay checkpoints.`);
} finally { await rm(root,{recursive:true,force:true}); }
