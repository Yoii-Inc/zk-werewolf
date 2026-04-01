import { shouldFetchGameInfoByRoomStatus } from "~~/hooks/useGameInfo";

describe("shouldFetchGameInfoByRoomStatus", () => {
  test("returns true for InProgress", () => {
    expect(shouldFetchGameInfoByRoomStatus("InProgress")).toBe(true);
  });

  test("returns true for Closed", () => {
    expect(shouldFetchGameInfoByRoomStatus("Closed")).toBe(true);
  });

  test("returns false for Open", () => {
    expect(shouldFetchGameInfoByRoomStatus("Open")).toBe(false);
  });

  test("returns false for Ready", () => {
    expect(shouldFetchGameInfoByRoomStatus("Ready")).toBe(false);
  });

  test("returns false for unknown values", () => {
    expect(shouldFetchGameInfoByRoomStatus("Unknown")).toBe(false);
    expect(shouldFetchGameInfoByRoomStatus(undefined)).toBe(false);
    expect(shouldFetchGameInfoByRoomStatus(null)).toBe(false);
  });
});
