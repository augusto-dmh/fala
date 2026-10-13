import { useEffect, useState } from "react";
import { commands, type MeetingTemplates } from "@/bindings";

/**
 * The notes templates, read again on mount and whenever the app gets the focus back: the person
 * edits the JSON in another app (no file watcher).
 */
export function useMeetingTemplates() {
  const [catalog, setCatalog] = useState<MeetingTemplates>({
    templates: [],
    errors: [],
  });
  useEffect(() => {
    const load = () =>
      void commands
        .meetingTemplates()
        .then((r) => r.status === "ok" && setCatalog(r.data));
    load();
    window.addEventListener("focus", load);
    return () => window.removeEventListener("focus", load);
  }, []);
  return [catalog, setCatalog] as const;
}
