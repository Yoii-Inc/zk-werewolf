import { NextResponse } from "next/server";
import fs from "fs";
import path from "path";

export async function GET(request: Request) {
  try {
    const { searchParams } = new URL(request.url);
    const playerId = searchParams.get("playerId");

    if (!playerId) {
      return NextResponse.json({ error: "Player ID is required" }, { status: 400 });
    }

    const filePath = path.join(process.cwd(), "data", `player_keys_${playerId}.json`);

    if (!fs.existsSync(filePath)) {
      return NextResponse.json({ error: "Key pair not found" }, { status: 404 });
    }

    const fileContent = fs.readFileSync(filePath, "utf8");
    const keyData = JSON.parse(fileContent);

    return NextResponse.json(keyData);
  } catch (error) {
    console.error("Error loading key pair:", error);
    return NextResponse.json({ error: "Failed to load key pair" }, { status: 500 });
  }
}
