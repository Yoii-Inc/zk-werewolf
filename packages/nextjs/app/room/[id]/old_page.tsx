// "use client";

// import { useEffect, useState } from "react";
// import type { NextPage } from "next";
// import LobbyList from "~~/app/game/LobbyList";

// // import { MetadataRoute } from 'next/server';
// // import { Metadata } from 'next';
// // import { getVillage } from '@/services/villageService'; // 村のデータを取得する関数
// // import { Village } from '@/types/village'; // 村のデータ型

// // export async function generateMetadata({ params }: { params: { id: string } }): Promise<Metadata> {
// //   const village = await getVillage(params.id);
// //   return {
// //     title: village ? `${village.name} - Village` : 'Village Not Found',
// //     description: village ? `Play in village ${village.name}` : 'Village not found',
// //   };
// // }

// // export default async function VillagePage({ params }: { params: { id: string } }) {
// //   const village = await getVillage(params.id);

// //   if (!village) {
// //     return <div>村が見つかりません</div>;
// //   }

// //   return (
// //     <div>
// //       <h1>{village.name}</h1>
// //       {/* 村の情報を表示 */}
// //       <p>ID: {village.id}</p>
// //       <p>最大プレイヤー数: {village.max_players}</p>
// //       {/* プレイするためのUIを追加 */}
// //       <button>ゲームに参加</button>
// //     </div>
// //   );
// // return (
// //     <div>
// //         <h1>Room</h1>
// //     </div>
// // )
// // }

// // export const metadata: MetadataRoute = {
// //   generateMetadata,
// // };

// // import LobbyList from "./LobbyList";

// export default function RoomPage({ params }: { params: { id: string } }): NextPage {
//   const [players, setPlayers] = useState(["Player 1", "Player 2", "Player 3"]);
//   const [gameStarted, setGameStarted] = useState(false);
//   const [gameState, setGameState] = useState({
//     phase: "夜", // '夜', '昼', 'ゲーム終了' など
//     roles: {}, // { 'Player 1': '人狼', 'Player 2': '村人', ... }
//     remainingTime: 60, // 秒
//   });
//   const currentPlayerRole = "村人"; // ログインユーザーの役職 (仮置き)

//   //   const village = await getVillage(params.id);
//   const village = [];

//   const startGame = () => {
//     setGameStarted(true);
//     const roles = ["人狼", "人狼", "村人"]; // 役職の例 (プレイヤー数に合わせて調整)
//     const shuffledRoles = roles.sort(() => Math.random() - 0.5);
//     const assignedRoles = {};
//     players.forEach((player, index) => {
//       assignedRoles[player] = shuffledRoles[index];
//     });
//     setGameState({ ...gameState, roles: assignedRoles, phase: "夜" });
//   };

//   const formatTime = (seconds: number): string => {
//     const minutes = Math.floor(seconds / 60);
//     const remainingSeconds = seconds % 60;
//     return `${minutes}:${remainingSeconds < 10 ? "0" : ""}${remainingSeconds}`;
//   };

//   const [chatMessages, setChatMessages] = useState([
//     { player: "Player 1", message: "おはようございます！" },
//     { player: "Player 2", message: "おはようございます！" },
//   ]);

//   const [apiResponse, setApiResponse] = useState(""); // APIレスポンスを格納する状態変数を追加

//   //   useEffect(() => {
//   //     const fetchData = async () => {
//   //       try {
//   //         const response = await fetch("http://127.0.0.1:8080/greet?name=Alice");
//   //         if (!response.ok) {
//   //           throw new Error(`HTTP error! status: ${response.status}`);
//   //         }
//   //         const text = await response.text();
//   //         setApiResponse(text); // レスポンスを状態変数に設定
//   //       } catch (error) {
//   //         console.error("Error fetching data:", error);
//   //         setApiResponse("APIリクエストに失敗しました。"); // エラーメッセージを設定
//   //       }
//   //     };

//   //     if (gameStarted) {
//   //       // ゲーム開始後にAPIリクエストを実行
//   //       fetchData();
//   //     }
//   //   }, [gameStarted]); // gameStartedが変更されたときにuseEffectが実行されるように依存関係に追加

//   const [websocket, setWebsocket] = useState<WebSocket | null>(null);
//   const [websocketStatus, setWebsocketStatus] = useState<string>("disconnected"); // WebSocket接続状態

//   //   useEffect(() => {
//   //     // WebSocket接続を確立
//   //     const ws = new WebSocket("ws://localhost:8080/api/room/ws"); // wsプロトコルを使用

//   //     ws.onopen = () => {
//   //       console.log("WebSocket接続が確立されました");
//   //     };

//   //     ws.onmessage = (event) => {
//   //       console.log("メッセージを受信しました:", event.data);
//   //       // サーバーからの応答を処理する
//   //     };

//   //     ws.onclose = () => {
//   //       console.log("WebSocket接続が閉じられました");
//   //       setWebsocket(null); // WebSocketをnullに設定
//   //     };

//   //     ws.onerror = (error) => {
//   //       console.error("WebSocketエラーが発生しました:", error);
//   //       setWebsocket(null); // WebSocketをnullに設定
//   //     };

//   //     setWebsocket(ws);

//   //     // クリーンアップ関数
//   //     return () => {
//   //       if (ws.readyState !== WebSocket.CLOSED) {
//   //         ws.close();
//   //       }
//   //     };
//   //   }, []);

//   const connectWebSocket = () => {
//     setWebsocketStatus("connecting");
//     const ws = new WebSocket("ws://localhost:8080/api/room/ws");

//     ws.onopen = () => {
//       console.log("WebSocket接続が確立されました");
//       setWebsocketStatus("connected");
//     };

//     ws.onmessage = (event) => {
//       console.log("メッセージを受信しました:", event.data);
//       // サーバーからの応答を処理する
//       setChatMessages((prevMessages) => [...prevMessages, { player: "Server", message: event.data }]);
//     };

//     ws.onclose = (event) => {
//       console.log("WebSocket接続が閉じられました", event);
//       setWebsocketStatus("disconnected");
//       setWebsocket(null);
//     };

//     ws.onerror = (error) => {
//       console.error("WebSocketエラーが発生しました:", error);
//       setWebsocketStatus("error");
//       setWebsocket(null);
//     };

//     setWebsocket(ws);
//   };

//   const disconnectWebSocket = () => {
//     if (websocket && websocket.readyState !== WebSocket.CLOSED) {
//       websocket.close();
//     }
//   };

//   useEffect(() => {
//     // クリーンアップ関数
//     return () => {
//       disconnectWebSocket();
//     };
//   }, []);

//   const sendMessage = () => {
//     const message = document.querySelector("textarea")?.value; // テキストエリアの値を取得
//     if (websocket && message) {
//       websocket.send(message);
//       const textarea = document.querySelector("textarea");
//       if (textarea) {
//         textarea.value = ""; // テキストエリアをクリア
//       }
//     } else {
//       if (!websocket) {
//         console.error("WebSocket接続が確立されていません。");
//       }
//       if (!message) {
//         console.error("メッセージが空です。");
//       }
//       console.error("WebSocket接続が確立されていません、またはメッセージが空です。");
//     }
//   };

//   return (
//     <div className="flex flex-row h-screen">
//       {/* ゲーム情報 */}
//       <div className="w-1/4 p-4 border-r border-gray-300">
//         <h1 className="text-4xl my-0">人狼ゲーム</h1>
//         <button onClick={startGame} disabled={gameStarted} className="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded">
//           ゲーム開始
//         </button>

//         <p>プレイヤー一覧</p>
//         <ul>
//           {players.map((player, index) => (
//             <li key={index}>{player}</li>
//           ))}
//         </ul>
//         {gameStarted && (
//           <div>
//             <p>現在のフェーズ: {gameState.phase}</p>
//             <p>あなたの役職: {currentPlayerRole}</p>
//             <p>残り時間: {formatTime(gameState.remainingTime)}</p>
//             {/* ゲーム進行部分 (プレースホルダー) */}
//           </div>
//         )}

//         <button onClick={connectWebSocket} disabled={websocketStatus === "connected" || websocketStatus === "connecting"} className="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded">
//           {websocketStatus === "connecting" ? "接続中..." : "WebSocket接続"}
//         </button>
//         <button onClick={disconnectWebSocket} disabled={websocketStatus === "disconnected"} className="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded">
//           WebSocket切断
//         </button>
//       </div>

//       {/* チャット */}
//       <div className="w-1/2 p-4 border-r border-gray-300">
//         <h2 className="text-2xl">チャットログ</h2>
//         <ul>
//           {chatMessages.map((msg, index) => (
//             <li key={index}>
//               {msg.player}: {msg.message}
//             </li>
//           ))}
//         </ul>
//         {/* メッセージ入力欄など */}
//         <textarea className="w-full h-24 p-2 border border-gray-300 rounded" placeholder="メッセージを入力"></textarea>
//         <button onClick={sendMessage} className="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded mt-2">送信</button>
//       </div>

//       {/* メモ欄 */}
//       <div className="w-1/4 p-4">
//         <h2 className="text-2xl">メモ</h2>
//         {/* メモ入力フォームなど */}
//       </div>
//     </div>
//   );
// }

// // export default WerewolfGame;
