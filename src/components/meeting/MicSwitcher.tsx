import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import { DEFAULT_DEVICE_NAME, deviceLabel } from "@/lib/utils/deviceLabel";
import { useSettings } from "../../hooks/useSettings";
import { Dropdown } from "../ui/Dropdown";

/** What the pill's muted-mic card sends to the main window (`open_meeting_mic_picker`). */
export const PICK_MIC_EVENT = "meeting-pick-mic";

/**
 * "Switch microphone" on the muted-mic strip: the microphone picked here is the one dictation
 * uses, and the meeting being recorded moves to it at once without stopping. The pill's card
 * opens the list.
 */
export function MicSwitcher() {
  const { t } = useTranslation();
  const {
    getSetting,
    updateSetting,
    isUpdating,
    audioDevices,
    refreshAudioDevices,
  } = useSettings();
  const [openSignal, setOpenSignal] = useState(0);
  useEffect(() => {
    const unlisten = listen(PICK_MIC_EVENT, () => {
      void refreshAudioDevices();
      setOpenSignal((n) => n + 1);
    });
    return () => void unlisten.then((stop) => stop());
  }, [refreshAudioDevices]);

  const selected = getSetting("selected_microphone");
  return (
    <span className="flex items-center gap-2">
      {t("meeting.recorder.switchMic")}
      <Dropdown
        options={audioDevices.map((device) => ({
          value: device.name,
          label: deviceLabel(device.name, t),
        }))}
        selectedValue={
          !selected || selected === "default" ? DEFAULT_DEVICE_NAME : selected
        }
        onSelect={(name) => void updateSetting("selected_microphone", name)}
        placeholder={t("settings.sound.microphone.placeholder")}
        disabled={isUpdating("selected_microphone")}
        onOpen={refreshAudioDevices}
        openSignal={openSignal}
      />
    </span>
  );
}
