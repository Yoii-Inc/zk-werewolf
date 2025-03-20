"use client";

import React from "react";
import type { GameLog } from "./types";
import { Calendar, Moon, Trophy, Users } from "lucide-react";

const mockLogs: GameLog[] = [
  {
    id: "1",
    date: new Date().toISOString(),
    roomName: "初心者歓迎！",
    players: 8,
    result: "村人陣営勝利",
    role: "村人",
  },
  {
    id: "2",
    date: new Date(Date.now() - 86400000).toISOString(), // 1日前
    roomName: "経験者のみ",
    players: 8,
    result: "人狼陣営勝利",
    role: "人狼",
  },
];

export default function RecordScreen() {
  return (
    <div className="p-6 max-w-4xl mx-auto">
      <div className="flex items-center gap-4 mb-8">
        <Moon size={32} className="text-indigo-600" />
        <h1 className="text-3xl font-bold text-indigo-900">ゲーム記録</h1>
      </div>

      <div className="bg-white/80 backdrop-blur-sm rounded-xl shadow-sm border border-indigo-50">
        <div className="grid gap-4 p-6">
          {mockLogs.map(log => (
            <div
              key={log.id}
              className="border border-indigo-100 rounded-xl p-6 hover:border-indigo-200 transition-colors"
            >
              <div className="flex justify-between items-center mb-4">
                <h2 className="text-xl font-semibold text-indigo-900">{log.roomName}</h2>
                <span
                  className={`px-4 py-1.5 rounded-full text-sm font-medium ${
                    log.result.includes("村人") ? "bg-green-50 text-green-700" : "bg-red-50 text-red-700"
                  }`}
                >
                  {log.result}
                </span>
              </div>

              <div className="grid grid-cols-3 gap-6 text-indigo-700">
                <div className="flex items-center gap-2">
                  <Calendar size={18} />
                  <span>{new Date(log.date).toLocaleDateString()}</span>
                </div>
                <div className="flex items-center gap-2">
                  <Users size={18} />
                  <span>{log.players}人参加</span>
                </div>
                <div className="flex items-center gap-2">
                  <Trophy size={18} />
                  <span>役職: {log.role}</span>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
