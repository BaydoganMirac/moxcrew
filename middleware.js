import { NextResponse } from "next/server";

const securityHeaders = {
  "Permissions-Policy": "camera=(), microphone=(), geolocation=()",
  "Referrer-Policy": "same-origin",
  "X-Content-Type-Options": "nosniff",
  "X-Frame-Options": "DENY",
};

function isAllowedHost(host) {
  if (!host) return false;

  const match = /^(?:localhost|127\.0\.0\.1)(?::([0-9]+))?$/i.exec(host);
  if (!match) return false;

  const port = match[1] ? Number(match[1]) : null;
  return port === null || (port >= 1 && port <= 65535);
}

export function middleware(request) {
  const response = isAllowedHost(request.headers.get("host"))
    ? NextResponse.next()
    : new NextResponse("Forbidden", { status: 403 });

  for (const [name, value] of Object.entries(securityHeaders)) {
    response.headers.set(name, value);
  }

  return response;
}
