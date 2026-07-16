import type { NextRequest } from "next/server";
import { NextResponse } from "next/server";
import { disconnectCloud, pairCloud } from "@/lib/cloud/sync.mjs";
import { readLocalForm } from "@/lib/runtime/local-form.mjs";

const SHAPES = {
  pair: ["action", "cloudUrl", "displayName", "pairingCode"],
  disconnect: ["action"],
};

export async function POST(request: NextRequest) {
  let form: Record<string, string>;
  try {
    form = await readLocalForm(request, SHAPES, { maximumBytes: 2_048 }) as Record<string, string>;
  } catch {
    return new NextResponse("Forbidden", { status: 403 });
  }

  try {
    if (form.action === "pair") {
      await pairCloud({
        cloudUrl: form.cloudUrl,
        displayName: form.displayName,
        pairingCode: form.pairingCode,
      });
    } else {
      await disconnectCloud();
    }
    return NextResponse.redirect(new URL("/", request.headers.get("origin")!), 303);
  } catch {
    return new NextResponse("Cloud pairing failed", { status: 400 });
  }
}
