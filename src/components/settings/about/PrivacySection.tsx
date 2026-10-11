import React, { useEffect, useState } from "react";
import { BaseDirectory, readTextFile } from "@tauri-apps/plugin-fs";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import { commands, type MeetingKeys } from "@/bindings";
import { useSettings } from "../../../hooks/useSettings";
import { PrivacyView } from "./PrivacyView";
import {
  DECISIONS_URL,
  meetingAudioDir,
  privacyInputs,
  privacyRows,
} from "./privacyModel";

/** The Privacy section of About: reads the current state, shows it, changes nothing. */
export const PrivacySection: React.FC = () => {
  const { settings } = useSettings();
  const [meetingKeys, setMeetingKeys] = useState<MeetingKeys | null>(null);
  const [mcpToml, setMcpToml] = useState<string | null>(null);
  const [appDir, setAppDir] = useState("");

  useEffect(() => {
    void commands
      .meetingKeys()
      .then((r) => r.status === "ok" && setMeetingKeys(r.data))
      .catch((e) => console.error("Failed to read the meeting keys:", e));
    // `fala-mcp` reads `mcp.toml` from the OS data folder, which is `$APPDATA` here.
    // Missing or unreadable: the MCP is off, as in `crates/mcp`.
    readTextFile("mcp.toml", { baseDir: BaseDirectory.AppData })
      .then(setMcpToml)
      .catch(() => setMcpToml(null));
    void commands
      .getAppDirPath()
      .then((r) => r.status === "ok" && setAppDir(r.data))
      .catch((e) => console.error("Failed to get the app directory:", e));
  }, []);

  const audioDir = meetingAudioDir(appDir);

  const openAudio = async () => {
    try {
      await revealItemInDir(audioDir);
    } catch {
      // No meeting recorded yet: the folder does not exist, so open the data folder.
      await commands.openAppDataDir();
    }
  };

  return (
    <PrivacyView
      rows={privacyRows(privacyInputs(settings, meetingKeys, mcpToml))}
      audioDir={audioDir}
      onOpenAudio={() => void openAudio()}
      onOpenDecisions={() => void openUrl(DECISIONS_URL)}
    />
  );
};
