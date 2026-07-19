import type { NextRequest } from "next/server";
import { NextResponse } from "next/server";
import { refreshObservations } from "@/lib/runtime/observe.mjs";
import { readLocalForm } from "@/lib/runtime/local-form.mjs";
import { reconcileWorkControlOwnership } from "@/lib/runtime/work-queue.mjs";
import { advanceAgentTeams } from "@/lib/runtime/team-orchestrator.mjs";

export async function POST(request: NextRequest) {
  try {
    await readLocalForm(request, []);
  } catch {
    return new NextResponse("Forbidden", { status: 403 });
  }
  try {
    await refreshObservations();
    await reconcileWorkControlOwnership();
    await advanceAgentTeams();
    return NextResponse.redirect(new URL("/", request.headers.get("origin")!), 303);
  } catch {
    return new NextResponse("Local observation failed", { status: 500 });
  }
}
