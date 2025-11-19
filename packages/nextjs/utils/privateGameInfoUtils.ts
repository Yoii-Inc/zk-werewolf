import { PrivateGameInfo } from "~~/types/game";

/**
 * セッションストレージからPrivateGameInfoを取得
 * @param roomId ルームID
 * @param playerId プレイヤーID
 * @returns PrivateGameInfo | null
 */
export const getPrivateGameInfo = (roomId: string, playerId: string): PrivateGameInfo | null => {
  try {
    const storedData = sessionStorage.getItem(`game_${roomId}_player_${playerId}`);
    if (!storedData) return null;
    return JSON.parse(storedData) as PrivateGameInfo;
  } catch (error) {
    console.error("PrivateGameInfo取得エラー:", error);
    return null;
  }
};

/**
 * セッションストレージにPrivateGameInfoを保存
 * @param roomId ルームID
 * @param privateGameInfo プライベートゲーム情報
 */
export const setPrivateGameInfo = (roomId: string, privateGameInfo: PrivateGameInfo): void => {
  try {
    sessionStorage.setItem(`game_${roomId}_player_${privateGameInfo.playerId}`, JSON.stringify(privateGameInfo));
  } catch (error) {
    console.error("PrivateGameInfo保存エラー:", error);
  }
};

/**
 * セッションストレージのPrivateGameInfoを更新
 * @param roomId ルームID
 * @param playerId プレイヤーID
 * @param updates 更新データ
 * @returns 更新後のPrivateGameInfo | null
 */
export const updatePrivateGameInfo = (
  roomId: string,
  playerId: string,
  updates: Partial<PrivateGameInfo>,
): PrivateGameInfo | null => {
  try {
    const currentInfo = getPrivateGameInfo(roomId, playerId);
    if (!currentInfo) return null;

    const updatedInfo = { ...currentInfo, ...updates };
    setPrivateGameInfo(roomId, updatedInfo);
    return updatedInfo;
  } catch (error) {
    console.error("PrivateGameInfo更新エラー:", error);
    return null;
  }
};

/**
 * hasActedフラグを更新
 * @param roomId ルームID
 * @param playerId プレイヤーID
 * @param hasActed アクション実行済みフラグ
 */
export const updateHasActed = (roomId: string, playerId: string, hasActed: boolean): void => {
  updatePrivateGameInfo(roomId, playerId, { hasActed });
};

/**
 * セッションストレージからPrivateGameInfoを削除
 * @param roomId ルームID
 * @param playerId プレイヤーID
 */
export const clearPrivateGameInfo = (roomId: string, playerId: string): void => {
  try {
    sessionStorage.removeItem(`game_${roomId}_player_${playerId}`);
  } catch (error) {
    console.error("PrivateGameInfo削除エラー:", error);
  }
};
