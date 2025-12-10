import { useEffect, useMemo, useState } from "react";
import { GameInputGenerator } from "~~/services/gameInputGenerator";
import { GameInfo } from "~~/types/game";

// ゲーム固有の暗号パラメータを管理するフック
// NOTE: gameInfo.crypto_parametersから暗号パラメータを取得します。
// 存在しない場合は静的ファイルから読み込むフォールバック処理を行います。
export const useGameCrypto = (roomId: string, gameInfo: GameInfo | null = null) => {
  const [cryptoParams, setCryptoParams] = useState<any | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchCryptoParams = async () => {
    if (!roomId) return;

    setLoading(true);
    setError(null);

    try {
      // gameInfo.crypto_parametersから暗号パラメータを取得
      if (gameInfo?.crypto_parameters) {
        console.log("Using crypto params from gameInfo");
        setCryptoParams(gameInfo.crypto_parameters);
        return;
      }

      // NOTE: 将来的にAPI経由で取得する場合は以下のコメントを解除
      // const response = await fetch(`${process.env.NEXT_PUBLIC_API_URL}/api/game/${roomId}/crypto-params`);
      // if (!response.ok) {
      //   throw new Error(`Failed to fetch crypto params: ${response.statusText}`);
      // }
      // const params = await response.json();
      // setCryptoParams(params);

      // gameInfoに存在しない場合はフォールバック処理
      throw new Error("crypto_parameters not found in gameInfo");
    } catch (err) {
      console.warn("Using static files for crypto params:", err);

      // フォールバック: 静的ファイルから読み込み（開発用）
      try {
        const fallbackParams = {
          pedersenParam: null, // GameInputGeneratorのloadCryptoParams()で読み込む
          elgamalParam: null,
          elgamalPublicKey: null,
          playerCommitments: [],
          gameId: roomId,
          createdAt: new Date().toISOString(),
        };

        setCryptoParams(fallbackParams);
      } catch (fallbackErr) {
        setError(fallbackErr instanceof Error ? fallbackErr.message : "Failed to load crypto params");
      }
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchCryptoParams();
  }, [roomId, gameInfo]);

  return {
    cryptoParams,
    loading,
    error,
    refetch: fetchCryptoParams,
  };
};

// ゲーム入力生成を管理するフック
export const useGameInputGenerator = (roomId: string, username: string, gameInfo: GameInfo | null = null) => {
  const { cryptoParams } = useGameCrypto(roomId, gameInfo);
  const [isReady, setIsReady] = useState(false);

  const inputGenerator = useMemo(() => {
    if (!username || !roomId) {
      return null;
    }

    const generator = new GameInputGenerator(roomId, username, gameInfo, cryptoParams);
    return generator;
  }, [roomId, username, gameInfo, cryptoParams]);

  // ゲーム情報が更新されたときに入力生成器を更新
  useEffect(() => {
    if (inputGenerator && gameInfo) {
      inputGenerator.updateGameInfo(gameInfo);
    }
  }, [inputGenerator, gameInfo]);

  // ランダムネスの初期化
  useEffect(() => {
    const initRandomness = async () => {
      if (!inputGenerator) return;

      try {
        await inputGenerator.initializeRandomness();
        setIsReady(true);
        console.log("InputGenerator randomness initialized");
      } catch (error) {
        console.error("Failed to initialize randomness:", error);
        setIsReady(false);
      }
    };

    initRandomness();
  }, [inputGenerator]);

  // 暗号パラメータが更新されたときに入力生成器を更新
  useEffect(() => {
    if (inputGenerator && cryptoParams) {
      inputGenerator.updateCryptoParams(cryptoParams);
    }
  }, [inputGenerator, cryptoParams]);

  return {
    inputGenerator,
    cryptoParams,
    isReady: isReady && inputGenerator?.isRandomnessInitialized(),
  };
};
