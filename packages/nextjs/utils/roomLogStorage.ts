const clientMessagesKey = (roomId: string) => `client_messages_${roomId}`;
const wsLastEventIdKey = (roomId: string) => `ws_last_event_id_${roomId}`;
const privateGameInfoKey = (roomId: string, playerId: string) => `game_${roomId}_player_${playerId}`;
const divinationLogsKey = (roomId: string, playerId: string) => `divination_logs_${roomId}_${playerId}`;

const removeKeysByPrefix = (storage: Storage, prefixes: string[]) => {
  const keysToRemove: string[] = [];
  for (let i = 0; i < storage.length; i += 1) {
    const key = storage.key(i);
    if (!key) continue;
    if (prefixes.some(prefix => key.startsWith(prefix))) {
      keysToRemove.push(key);
    }
  }

  keysToRemove.forEach(key => storage.removeItem(key));
};

export const clearRoomScopedLogs = (roomId: string, playerId?: string) => {
  if (typeof window === "undefined" || !roomId) return;

  localStorage.removeItem(clientMessagesKey(roomId));
  sessionStorage.removeItem(wsLastEventIdKey(roomId));

  if (playerId) {
    localStorage.removeItem(divinationLogsKey(roomId, playerId));
    sessionStorage.removeItem(privateGameInfoKey(roomId, playerId));
  }

  removeKeysByPrefix(localStorage, [
    `divination_target_${roomId}`,
    `divination_target_name_${roomId}`,
    `pending_divination_target_${roomId}`,
  ]);
};
