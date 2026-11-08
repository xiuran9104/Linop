import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: mocks.invoke,
}));

describe("mq users/permissions tauri API", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.unstubAllGlobals();
  });

  it("invokes mq_list_users with only the connection id", async () => {
    const { mqListUsers } = await import("@/lib/backend/mq-tauri");
    mocks.invoke.mockResolvedValue([{ name: "linop-db-app", tags: ["administrator"] }]);

    const result = await mqListUsers("conn-1");

    expect(mocks.invoke).toHaveBeenCalledWith("mq_list_users", { connectionId: "conn-1" });
    expect(result).toHaveLength(1);
    expect(result[0]?.name).toBe("linop-db-app");
    expect(result[0]?.tags).toEqual(["administrator"]);
  });

  it("invokes mq_create_user with name, password and optional tags", async () => {
    const { mqCreateUser } = await import("@/lib/backend/mq-tauri");
    mocks.invoke.mockResolvedValue(undefined);

    await mqCreateUser("conn-1", "linop-db-app", "secret", ["monitoring", "management"]);
    expect(mocks.invoke).toHaveBeenCalledWith("mq_create_user", { connectionId: "conn-1", name: "linop-db-app", password: "secret", tags: ["monitoring", "management"] });

    await mqCreateUser("conn-1", "linop-db-app2", "secret2");
    expect(mocks.invoke).toHaveBeenCalledWith("mq_create_user", { connectionId: "conn-1", name: "linop-db-app2", password: "secret2", tags: undefined });
  });

  it("invokes mq_delete_user with the user name", async () => {
    const { mqDeleteUser } = await import("@/lib/backend/mq-tauri");
    mocks.invoke.mockResolvedValue(undefined);

    await mqDeleteUser("conn-1", "linop-db-app");

    expect(mocks.invoke).toHaveBeenCalledWith("mq_delete_user", { connectionId: "conn-1", name: "linop-db-app" });
  });

  it("invokes mq_list_user_permissions with flattened filters", async () => {
    const { mqListUserPermissions } = await import("@/lib/backend/mq-tauri");
    mocks.invoke.mockResolvedValue([{ user: "linop-db-app", vhost: "/", configure: ".*", write: ".*", read: ".*" }]);

    const result = await mqListUserPermissions("conn-1", { virtualHost: "/" });
    expect(mocks.invoke).toHaveBeenCalledWith("mq_list_user_permissions", { connectionId: "conn-1", virtualHost: "/", user: undefined, allVhosts: undefined });
    expect(result[0]?.vhost).toBe("/");

    await mqListUserPermissions("conn-1", { allVhosts: true, user: "linop-db-app" });
    expect(mocks.invoke).toHaveBeenCalledWith("mq_list_user_permissions", { connectionId: "conn-1", virtualHost: undefined, user: "linop-db-app", allVhosts: true });

    await mqListUserPermissions("conn-1");
    expect(mocks.invoke).toHaveBeenCalledWith("mq_list_user_permissions", { connectionId: "conn-1", virtualHost: undefined, user: undefined, allVhosts: undefined });
  });

  it("invokes mq_grant_user_permission with optional patterns", async () => {
    const { mqGrantUserPermission } = await import("@/lib/backend/mq-tauri");
    mocks.invoke.mockResolvedValue(undefined);

    await mqGrantUserPermission("conn-1", "linop-db-app", "/", { configure: "^linop-db-", write: ".*", read: ".*" });
    expect(mocks.invoke).toHaveBeenCalledWith("mq_grant_user_permission", { connectionId: "conn-1", user: "linop-db-app", virtualHost: "/", configure: "^linop-db-", write: ".*", read: ".*" });

    await mqGrantUserPermission("conn-1", "linop-db-app", "/");
    expect(mocks.invoke).toHaveBeenCalledWith("mq_grant_user_permission", { connectionId: "conn-1", user: "linop-db-app", virtualHost: "/", configure: undefined, write: undefined, read: undefined });
  });

  it("invokes mq_revoke_user_permission with user and vhost", async () => {
    const { mqRevokeUserPermission } = await import("@/lib/backend/mq-tauri");
    mocks.invoke.mockResolvedValue(undefined);

    await mqRevokeUserPermission("conn-1", "linop-db-app", "/");

    expect(mocks.invoke).toHaveBeenCalledWith("mq_revoke_user_permission", { connectionId: "conn-1", user: "linop-db-app", virtualHost: "/" });
  });
});

describe("mq users/permissions HTTP API", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.unstubAllGlobals();
  });

  function stubFetch() {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      json: vi.fn().mockResolvedValue([]),
    });
    vi.stubGlobal("fetch", fetchMock);
    return fetchMock;
  }

  function lastCall(fetchMock: ReturnType<typeof stubFetch>): { url: string; body: Record<string, unknown> } {
    const [url, init] = fetchMock.mock.calls.at(-1) as [string, RequestInit];
    return { url, body: JSON.parse(String(init.body)) as Record<string, unknown> };
  }

  it("posts to the users endpoints", async () => {
    const fetchMock = stubFetch();
    const { mqListUsers, mqCreateUser, mqDeleteUser } = await import("@/lib/backend/mq-http");

    await mqListUsers("conn-1");
    expect(lastCall(fetchMock)).toEqual({ url: "/api/mq/users/list", body: { connectionId: "conn-1" } });

    await mqCreateUser("conn-1", "linop-db-app", "secret", ["monitoring"]);
    expect(lastCall(fetchMock)).toEqual({ url: "/api/mq/users/create", body: { connectionId: "conn-1", name: "linop-db-app", password: "secret", tags: ["monitoring"] } });

    await mqDeleteUser("conn-1", "linop-db-app");
    expect(lastCall(fetchMock)).toEqual({ url: "/api/mq/users/delete", body: { connectionId: "conn-1", name: "linop-db-app" } });
  });

  it("posts to the user-permissions endpoints", async () => {
    const fetchMock = stubFetch();
    const { mqListUserPermissions, mqGrantUserPermission, mqRevokeUserPermission } = await import("@/lib/backend/mq-http");

    await mqListUserPermissions("conn-1", { virtualHost: "/" });
    expect(lastCall(fetchMock)).toEqual({ url: "/api/mq/user-permissions/list", body: { connectionId: "conn-1", virtualHost: "/", user: undefined, allVhosts: undefined } });

    await mqGrantUserPermission("conn-1", "linop-db-app", "/", { configure: "^linop-db-" });
    expect(lastCall(fetchMock)).toEqual({ url: "/api/mq/user-permissions/grant", body: { connectionId: "conn-1", user: "linop-db-app", virtualHost: "/", configure: "^linop-db-", write: undefined, read: undefined } });

    await mqRevokeUserPermission("conn-1", "linop-db-app", "/");
    expect(lastCall(fetchMock)).toEqual({ url: "/api/mq/user-permissions/revoke", body: { connectionId: "conn-1", user: "linop-db-app", virtualHost: "/" } });
  });

  it("surfaces HTTP errors with the response detail", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: false,
        status: 500,
        text: vi.fn().mockResolvedValue("cannot delete the current connection user"),
      }),
    );
    const { mqDeleteUser } = await import("@/lib/backend/mq-http");

    await expect(mqDeleteUser("conn-1", "jjsd")).rejects.toThrow("cannot delete the current connection user");
  });
});
