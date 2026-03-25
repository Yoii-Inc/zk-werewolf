"use client";

import type { ReactNode } from "react";
import { BookOpen, Eye, MessageSquare, Moon, Shield, Skull, Users, Vote } from "lucide-react";

type InfoCard = {
  title: string;
  description: string;
  icon: ReactNode;
};

const roleCards: InfoCard[] = [
  {
    title: "Villager",
    description: "Find the werewolves through discussion and voting. Work with others and track inconsistencies.",
    icon: <Users className="h-5 w-5" />,
  },
  {
    title: "Seer",
    description: "During night, choose one player to inspect. Use your information carefully in daytime discussions.",
    icon: <Eye className="h-5 w-5" />,
  },
  {
    title: "Werewolf",
    description: "Coordinate with your team, blend in during discussion, and eliminate villagers at night.",
    icon: <Skull className="h-5 w-5" />,
  },
];

const phaseCards: InfoCard[] = [
  {
    title: "Night",
    description: "Night actions are submitted. Werewolves choose targets and special actions are processed.",
    icon: <Moon className="h-5 w-5" />,
  },
  {
    title: "DivinationProcessing",
    description: "The system processes divination-related computation results before discussion starts.",
    icon: <Shield className="h-5 w-5" />,
  },
  {
    title: "Discussion",
    description: "Players share information, claim roles, and build consensus about suspicious players.",
    icon: <MessageSquare className="h-5 w-5" />,
  },
  {
    title: "Voting",
    description: "Each alive player votes. Voting outcomes are aggregated for the current day.",
    icon: <Vote className="h-5 w-5" />,
  },
  {
    title: "Result",
    description: "The current round result is shown and the game proceeds or ends based on game state.",
    icon: <BookOpen className="h-5 w-5" />,
  },
];

const playSteps = [
  "Create a room from Home, set room options, and share the room with other players.",
  "Join a room and make sure every participant sets Ready before starting.",
  "Follow each phase and submit actions within the timer.",
  "Use discussion effectively, then vote carefully in the Voting phase.",
  "Review the result and continue the next cycle until a win condition is reached.",
];

const troubleshootingTips = [
  {
    title: "Cannot start game",
    body: "The game requires at least 4 players. Also make sure all players are marked as ready.",
  },
  {
    title: "Realtime updates look stale",
    body: "Refresh the room page and reconnect to the room if WebSocket updates are delayed.",
  },
  {
    title: "API connection issues",
    body: "Check backend availability at /health and verify NEXT_PUBLIC_API_URL / NEXT_PUBLIC_WS_URL settings.",
  },
];

const cardClass =
  "rounded-2xl border border-slate-200 bg-white p-5 shadow-sm transition hover:shadow-md hover:border-slate-300";

const HelpPage = () => {
  return (
    <div className="mx-auto w-full max-w-6xl px-4 py-8 md:px-6 md:py-10">
      <section className="rounded-3xl border border-slate-200 bg-gradient-to-br from-slate-50 via-white to-indigo-50 p-6 md:p-8 shadow-sm">
        <p className="text-sm font-semibold uppercase tracking-wide text-indigo-700">Game Guide</p>
        <h1 className="mt-2 text-3xl md:text-4xl font-bold text-slate-900">How To Play ZK Werewolf</h1>
        <p className="mt-3 text-slate-700 leading-relaxed">
          This page explains roles, phase flow, and common checks so new players can join a room and play smoothly.
        </p>
      </section>

      <section className="mt-8">
        <h2 className="text-2xl font-semibold text-slate-900">Roles</h2>
        <div className="mt-4 grid gap-4 md:grid-cols-3">
          {roleCards.map(card => (
            <article key={card.title} className={cardClass}>
              <div className="flex items-center gap-2 text-indigo-700">
                {card.icon}
                <h3 className="m-0 text-lg font-semibold text-slate-900">{card.title}</h3>
              </div>
              <p className="mt-3 text-sm text-slate-700 leading-relaxed">{card.description}</p>
            </article>
          ))}
        </div>
      </section>

      <section className="mt-8">
        <h2 className="text-2xl font-semibold text-slate-900">Phase Flow</h2>
        <p className="mt-2 text-slate-700">
          Night -&gt; DivinationProcessing -&gt; Discussion -&gt; Voting -&gt; Result
        </p>
        <div className="mt-4 grid gap-4 md:grid-cols-2 xl:grid-cols-5">
          {phaseCards.map(card => (
            <article key={card.title} className={cardClass}>
              <div className="flex items-center gap-2 text-indigo-700">
                {card.icon}
                <h3 className="m-0 text-base font-semibold text-slate-900">{card.title}</h3>
              </div>
              <p className="mt-3 text-sm text-slate-700 leading-relaxed">{card.description}</p>
            </article>
          ))}
        </div>
      </section>

      <section className="mt-8 grid gap-4 lg:grid-cols-2">
        <article className={cardClass}>
          <h2 className="text-2xl font-semibold text-slate-900">Basic Play Steps</h2>
          <ol className="mt-4 list-decimal pl-5 text-slate-700 space-y-2">
            {playSteps.map(step => (
              <li key={step} className="leading-relaxed">
                {step}
              </li>
            ))}
          </ol>
        </article>

        <article className={cardClass}>
          <h2 className="text-2xl font-semibold text-slate-900">Common Troubleshooting</h2>
          <div className="mt-4 space-y-4">
            {troubleshootingTips.map(tip => (
              <div key={tip.title} className="rounded-xl border border-slate-200 bg-slate-50 p-4">
                <h3 className="m-0 text-base font-semibold text-slate-900">{tip.title}</h3>
                <p className="mt-2 text-sm text-slate-700 leading-relaxed">{tip.body}</p>
              </div>
            ))}
          </div>
        </article>
      </section>
    </div>
  );
};

export default HelpPage;
