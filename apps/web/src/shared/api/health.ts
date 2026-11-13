export type ServiceReadiness = "available" | "unavailable";

export async function checkServiceReadiness(
  signal: AbortSignal,
): Promise<ServiceReadiness> {
  const response = await fetch("/api/health/ready", { signal });
  return response.ok ? "available" : "unavailable";
}
