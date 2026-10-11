import { useTranslation } from "react-i18next";
import { Button } from "../ui/Button";

export interface ConsentNoticeProps {
  onAccept: () => void;
  onCancel: () => void;
}

/** The third-party notice of ADR-0005 and ADR-0016, shown before the first recording. */
export function ConsentNotice({ onAccept, onCancel }: ConsentNoticeProps) {
  const { t } = useTranslation();
  return (
    <div role="alertdialog" className="space-y-3 p-4">
      <h3 className="text-sm font-semibold">{t("meeting.consent.title")}</h3>
      <p className="text-sm">{t("meeting.consent.body")}</p>
      <div className="flex gap-2">
        <Button variant="primary" onClick={onAccept}>
          {t("meeting.consent.accept")}
        </Button>
        <Button variant="secondary" onClick={onCancel}>
          {t("meeting.consent.cancel")}
        </Button>
      </div>
    </div>
  );
}
