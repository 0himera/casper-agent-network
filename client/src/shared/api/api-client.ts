const getBaseUrl = () => {
  if (typeof window !== "undefined") return "";
  return process.env.NEXT_PUBLIC_API_URL || process.env.API_URL || "http://localhost:8080";
};

class ApiError extends Error {
  constructor(public status: number, message: string) {
    super(message);
    this.name = "ApiError";
  }
}

async function request<T>(path: string, options?: RequestInit): Promise<T> {
  const url = `${getBaseUrl()}${path}`;
  const res = await fetch(url, {
    ...options,
    headers: { "Content-Type": "application/json", ...options?.headers },
  });

  if (!res.ok) {
    if (res.status === 402) {
      return [] as unknown as T;
    }
    const text = await res.text().catch(() => "Unknown error");
    throw new ApiError(res.status, text);
  }

  const text = await res.text();
  if (!text) return {} as T;
  
  try {
    return JSON.parse(text);
  } catch (e) {
    return text as unknown as T;
  }
}

export function apiGet<T>(path: string): Promise<T> {
  return request<T>(path);
}

export function apiPost<T>(path: string, body: unknown, headers?: Record<string, string>): Promise<T> {
  return request<T>(path, { method: "POST", body: JSON.stringify(body), headers });
}

export function apiPatch<T>(path: string, body: unknown): Promise<T> {
  return request<T>(path, { method: "PATCH", body: JSON.stringify(body) });
}
