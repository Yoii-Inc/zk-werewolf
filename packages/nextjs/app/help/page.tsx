"use client";

import { useState } from "react";
import Link from "next/link";

const HelpPage = () => {
  const [selectedSection, setSelectedSection] = useState<string | null>(null);

  const sections = [
    {
      title: "Getting Started",
      content: (
        <>
          <p>Welcome to Scaffold-ETH! This guide will help you get started.</p>
          <ol>
            <li>
              Clone the repository: <code>git clone https://github.com/scaffold-eth/scaffold-eth</code>
            </li>
            <li>
              Install dependencies: <code>npm install</code> or <code>yarn install</code>
            </li>
            <li>
              Start the development server: <code>npm run dev</code> or <code>yarn dev</code>
            </li>
          </ol>
        </>
      ),
    },
    {
      title: "Smart Contract Development",
      content: (
        <>
          <p>Learn how to develop and deploy smart contracts.</p>
          <ul>
            <li>
              <Link href="/docs/smart-contracts">Smart Contract Documentation</Link>
            </li>
          </ul>
        </>
      ),
    },
    {
      title: "Frontend Development",
      content: (
        <>
          <p>Learn how to develop and deploy the frontend application.</p>
          <ul>
            <li>
              <Link href="/docs/frontend">Frontend Documentation</Link>
            </li>
          </ul>
        </>
      ),
    },
    {
      title: "Troubleshooting",
      content: (
        <>
          <p>Common issues and solutions.</p>
          <ul>
            <li>If you encounter errors, check the console for details.</li>
            <li>
              Refer to the <Link href="https://github.com/scaffold-eth/scaffold-eth/issues">GitHub issues</Link> for
              known problems.
            </li>
          </ul>
        </>
      ),
    },
  ];

  return (
    <div className="container mx-auto p-4">
      <h1 className="text-3xl font-bold mb-4">Scaffold-ETH Help</h1>
      <ul>
        {sections.map(section => (
          <li key={section.title} className="mb-2 cursor-pointer" onClick={() => setSelectedSection(section.title)}>
            {section.title}
          </li>
        ))}
      </ul>
      {selectedSection && (
        <div className="mt-4 border p-4 rounded">
          <h2>{selectedSection}</h2>
          {sections.find(section => section.title === selectedSection)?.content}
        </div>
      )}
    </div>
  );
};

export default HelpPage;
