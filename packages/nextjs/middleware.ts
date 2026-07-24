import { NextResponse } from "next/server";
import type { NextRequest } from "next/server";

export function middleware(request: NextRequest) {
  const userAgent = request.headers.get("user-agent") ?? "-";

  // ALB health checks hit "/" every 30s; keep them out of the access log.
  if (userAgent.startsWith("ELB-HealthChecker")) {
    return NextResponse.next();
  }

  // Behind ALB + ECS Fargate (awsvpc) networking, request.ip is not reliable
  // in self-hosted Next.js. ALB is the first proxy hop and prepends the real
  // client IP to X-Forwarded-For.
  const forwardedFor = request.headers.get("x-forwarded-for");
  const clientIp = forwardedFor?.split(",")[0]?.trim() ?? "-";

  // Structured JSON to stdout so CloudWatch Logs Insights can query fields
  // directly, same shape style as the backend. Deliberately no cookies, no
  // Authorization header, and no query string (only the path) to avoid
  // leaking tokens into logs.
  console.log(
    JSON.stringify({
      timestamp: new Date().toISOString(),
      level: "INFO",
      message: "request received",
      method: request.method,
      path: request.nextUrl.pathname,
      user_agent: userAgent,
      client_ip: clientIp,
    }),
  );

  return NextResponse.next();
}

export const config = {
  matcher: [
    // Skip Next's own static/image assets (high volume, not interesting for
    // access analysis). Everything else - including bot-scanned paths like
    // /wp-admin, /.env, /favicon.ico - is still logged.
    "/((?!_next/static|_next/image).*)",
  ],
};
