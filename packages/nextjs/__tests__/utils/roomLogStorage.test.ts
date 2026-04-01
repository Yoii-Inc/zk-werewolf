import { clearRoomScopedLogs } from "~~/utils/roomLogStorage";

class StorageMock implements Storage {
  private store = new Map<string, string>();

  get length(): number {
    return this.store.size;
  }

  clear(): void {
    this.store.clear();
  }

  getItem(key: string): string | null {
    return this.store.has(key) ? this.store.get(key)! : null;
  }

  key(index: number): string | null {
    return Array.from(this.store.keys())[index] ?? null;
  }

  removeItem(key: string): void {
    this.store.delete(key);
  }

  setItem(key: string, value: string): void {
    this.store.set(key, value);
  }
}

describe("clearRoomScopedLogs", () => {
  beforeEach(() => {
    (global as any).window = {};
    (global as any).localStorage = new StorageMock();
    (global as any).sessionStorage = new StorageMock();
  });

  it("clears room scoped keys and keeps other rooms intact", () => {
    localStorage.setItem("client_messages_100", "old-messages");
    localStorage.setItem("client_messages_200", "other-room");
    localStorage.setItem("divination_logs_100_player-a", "logs-a");
    localStorage.setItem("divination_logs_100_player-b", "logs-b");
    localStorage.setItem("divination_target_100_player-a", "target");
    localStorage.setItem("divination_target_name_100_player-a", "target-name");
    localStorage.setItem("pending_divination_target_100_player-a", "pending-target");
    localStorage.setItem("divination_target_200_player-a", "other-target");
    sessionStorage.setItem("ws_last_event_id_100", "42");
    sessionStorage.setItem("ws_last_event_id_200", "99");
    sessionStorage.setItem("game_100_player_player-a", "private-game-a");
    sessionStorage.setItem("game_100_player_player-b", "private-game-b");

    clearRoomScopedLogs("100", "player-a");

    expect(localStorage.getItem("client_messages_100")).toBeNull();
    expect(localStorage.getItem("divination_logs_100_player-a")).toBeNull();
    expect(localStorage.getItem("divination_target_100_player-a")).toBeNull();
    expect(localStorage.getItem("divination_target_name_100_player-a")).toBeNull();
    expect(localStorage.getItem("pending_divination_target_100_player-a")).toBeNull();
    expect(sessionStorage.getItem("ws_last_event_id_100")).toBeNull();
    expect(sessionStorage.getItem("game_100_player_player-a")).toBeNull();

    expect(localStorage.getItem("client_messages_200")).toBe("other-room");
    expect(localStorage.getItem("divination_target_200_player-a")).toBe("other-target");
    expect(localStorage.getItem("divination_logs_100_player-b")).toBe("logs-b");
    expect(sessionStorage.getItem("ws_last_event_id_200")).toBe("99");
    expect(sessionStorage.getItem("game_100_player_player-b")).toBe("private-game-b");
  });

  it("keeps player scoped keys when playerId is omitted", () => {
    localStorage.setItem("divination_logs_300_player-c", "logs-c");
    sessionStorage.setItem("game_300_player_player-c", "private-game-c");

    clearRoomScopedLogs("300");

    expect(localStorage.getItem("divination_logs_300_player-c")).toBe("logs-c");
    expect(sessionStorage.getItem("game_300_player_player-c")).toBe("private-game-c");
  });
});
