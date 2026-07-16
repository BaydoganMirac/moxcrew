import type { NextRequest } from "next/server";
import { NextResponse } from "next/server";
import { triggerCloudSync } from "@/lib/cloud/sync.mjs";

export async function POST(request: NextRequest) {
  const token = process.env.PATCHFLEET_SYNC_TOKEN;
  const length = request.headers.get("content-length");
  if (
    !token ||
    request.headers.get("authorization") !== `Bearer ${token}` ||
    request.headers.has("origin") ||
    request.headers.has("transfer-encoding") ||
    (length !== null && length !== "0")
  ) {
    return new NextResponse("Forbidden", { status: 403 });
  }
  const sync = triggerCloudSync();
  if (!sync.accepted) return NextResponse.json({ kind: "in_progress" }, { status: 202 });
  return NextResponse.json(await sync.result);
}
