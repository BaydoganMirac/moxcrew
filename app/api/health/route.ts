import { NextResponse } from "next/server";

export const dynamic = "force-dynamic";

export function GET() {
  const runtimeId = process.env.PATCHFLEET_RUNTIME_ID ?? "manual";
  if (!/^(?:manual|runtime:[0-9a-f-]{36})$/i.test(runtimeId)) {
    return NextResponse.json({ schemaVersion: 1, available: false }, { status: 503 });
  }
  return NextResponse.json(
    { schemaVersion: 1, runtimeId },
    { headers: { "cache-control": "private, no-store" } },
  );
}
