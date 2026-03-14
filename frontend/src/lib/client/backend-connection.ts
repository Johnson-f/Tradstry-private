const BACKEND_URL =
  process.env.NEXT_PUBLIC_BACKEND_URL ?? "http://localhost:8080/graphql";

export function getBackendBaseUrl(): string {
  return BACKEND_URL.endsWith("/graphql")
    ? BACKEND_URL.slice(0, -"/graphql".length)
    : BACKEND_URL;
}

export type GraphQLFetcher = <T>(
  query: string,
  variables?: Record<string, unknown>,
) => Promise<T>;

const DEBUG_GRAPHQL = process.env.NODE_ENV !== "production";

export function createGraphQLFetcher(
  getToken: () => Promise<string | null>,
): GraphQLFetcher {
  return async <T>(
    query: string,
    variables?: Record<string, unknown>,
  ): Promise<T> => {
    const token = await getToken();
    const operationMatch = query.match(/\b(query|mutation)\s+([A-Za-z0-9_]+)/);
    const operationName = operationMatch?.[2] ?? "anonymous";

    if (DEBUG_GRAPHQL) {
      console.debug("[graphql] sending request", {
        operationName,
        url: BACKEND_URL,
        hasToken: Boolean(token),
        variables: variables ?? null,
      });
    }

    const res = await fetch(BACKEND_URL, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Accept: "application/graphql-response+json, application/json",
        ...(token ? { Authorization: `Bearer ${token}` } : {}),
      },
      body: JSON.stringify({ query, variables }),
    });

    if (!res.ok) {
      if (DEBUG_GRAPHQL) {
        console.error("[graphql] http error", {
          operationName,
          url: BACKEND_URL,
          status: res.status,
          statusText: res.statusText,
        });
      }
      throw new Error(
        `GraphQL request failed: ${res.status} ${res.statusText}`,
      );
    }

    const json = await res.json();
    if (json.errors?.length) {
      if (DEBUG_GRAPHQL) {
        console.error("[graphql] graphql error", {
          operationName,
          url: BACKEND_URL,
          errors: json.errors,
        });
      }
      throw new Error(json.errors[0].message);
    }

    if (DEBUG_GRAPHQL) {
      console.debug("[graphql] request succeeded", {
        operationName,
        url: BACKEND_URL,
      });
    }

    return json.data as T;
  };
}
