import { NextResponse } from "next/server";
import fs from "fs";
import path from "path";

export async function POST(request: Request) {
  try {
    const { playerId, keyData } = await request.json();

    const dataDir = path.join(process.cwd(), "data");
    if (!fs.existsSync(dataDir)) {
      fs.mkdirSync(dataDir, { recursive: true });
    }

    const filePath = path.join(dataDir, `player_keys_${playerId}.json`);
    fs.writeFileSync(filePath, JSON.stringify(keyData, null, 2));

    return NextResponse.json({ success: true });
  } catch (error) {
    console.error("Error saving key pair:", error);
    return NextResponse.json({ error: "Failed to save key pair" }, { status: 500 });
  }
}
