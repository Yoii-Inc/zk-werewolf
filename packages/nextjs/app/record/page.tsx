"use client";

import React from "react";
import { Moon } from "lucide-react";

export default function RecordScreen() {
  return (
    <div className="p-6 max-w-4xl mx-auto">
      <div className="flex items-center gap-4 mb-8">
        <Moon size={32} className="text-indigo-600" />
        <h1 className="text-3xl font-bold text-indigo-900">Record</h1>
      </div>

      <div className="bg-white/80 backdrop-blur-sm rounded-xl shadow-sm border border-indigo-50">
        <div className="p-8">
          <h2 className="text-xl font-semibold text-indigo-900 mb-3">Coming Soon</h2>
          <p className="text-indigo-800 leading-relaxed">
            This page is currently under development. In a future update, you will be able to view past game results and
            open the corresponding contract details for each match.
          </p>
        </div>
      </div>
    </div>
  );
}
