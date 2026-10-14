/**
 * Format a date string or timestamp to a localized date and time string
 * @param timestamp - Unix timestamp in seconds (as string)
 * @param locale - BCP 47 language tag (e.g., 'en', 'es', 'fr')
 * @returns Formatted date string
 */
export const formatDateTime = (timestamp: string, locale: string): string => {
  try {
    // Convert Unix timestamp (seconds) to milliseconds
    const timestampMs = parseInt(timestamp, 10) * 1000;
    const date = new Date(timestampMs);

    // Check if date is valid
    if (isNaN(date.getTime())) {
      return timestamp; // Return original if invalid
    }

    return new Intl.DateTimeFormat(locale, {
      year: "numeric",
      month: "long",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    }).format(date);
  } catch (error) {
    console.error("Failed to format date:", error);
    return timestamp; // Fallback to original timestamp
  }
};

/**
 * Format a date string or timestamp to a localized date string (no time)
 * @param timestamp - Unix timestamp in seconds (as string)
 * @param locale - BCP 47 language tag (e.g., 'en', 'es', 'fr')
 * @returns Formatted date string
 */
export const formatDate = (timestamp: string, locale: string): string => {
  try {
    // Convert Unix timestamp (seconds) to milliseconds
    const timestampMs = parseInt(timestamp, 10) * 1000;
    const date = new Date(timestampMs);

    // Check if date is valid
    if (isNaN(date.getTime())) {
      return timestamp; // Return original if invalid
    }

    return new Intl.DateTimeFormat(locale, {
      year: "numeric",
      month: "long",
      day: "numeric",
    }).format(date);
  } catch (error) {
    console.error("Failed to format date:", error);
    return timestamp; // Fallback to original timestamp
  }
};

/**
 * Format a date string or timestamp to a relative time string (e.g., "2 hours ago")
 * @param timestamp - Unix timestamp in seconds (as string)
 * @param locale - BCP 47 language tag (e.g., 'en', 'es', 'fr')
 * @returns Relative time string
 */
export const formatRelativeTime = (
  timestamp: string,
  locale: string,
): string => {
  try {
    // Convert Unix timestamp (seconds) to milliseconds
    const timestampMs = parseInt(timestamp, 10) * 1000;
    const date = new Date(timestampMs);
    const now = new Date();

    // Check if date is valid
    if (isNaN(date.getTime())) {
      return timestamp; // Return original if invalid
    }

    const diffInSeconds = Math.floor((now.getTime() - date.getTime()) / 1000);

    // Use Intl.RelativeTimeFormat for proper localization
    const rtf = new Intl.RelativeTimeFormat(locale, { numeric: "auto" });

    // Less than a minute
    if (diffInSeconds < 60) {
      return rtf.format(-diffInSeconds, "second");
    }

    // Less than an hour
    const diffInMinutes = Math.floor(diffInSeconds / 60);
    if (diffInMinutes < 60) {
      return rtf.format(-diffInMinutes, "minute");
    }

    // Less than a day
    const diffInHours = Math.floor(diffInMinutes / 60);
    if (diffInHours < 24) {
      return rtf.format(-diffInHours, "hour");
    }

    // Less than a week
    const diffInDays = Math.floor(diffInHours / 24);
    if (diffInDays < 7) {
      return rtf.format(-diffInDays, "day");
    }

    // Less than a month (30 days)
    if (diffInDays < 30) {
      const diffInWeeks = Math.floor(diffInDays / 7);
      return rtf.format(-diffInWeeks, "week");
    }

    // Less than a year
    if (diffInDays < 365) {
      const diffInMonths = Math.floor(diffInDays / 30);
      return rtf.format(-diffInMonths, "month");
    }

    // More than a year
    const diffInYears = Math.floor(diffInDays / 365);
    return rtf.format(-diffInYears, "year");
  } catch (error) {
    console.error("Failed to format relative time:", error);
    return formatDateTime(timestamp, locale); // Fallback to absolute time
  }
};

// One clock formatter per locale: lists format a time per row on every render.
const clocks = new Map<string, Intl.DateTimeFormat>();

/**
 * The local hour and minute on one short line: no leading zero on a 12-hour clock ("3:50 PM"),
 * two digits on a 24-hour one ("07:05").
 */
export const formatClock = (date: Date, locale: string): string => {
  let clock = clocks.get(locale);
  if (!clock) {
    const twelve =
      new Intl.DateTimeFormat(locale, { hour: "numeric" }).resolvedOptions()
        .hour12 === true;
    clock = new Intl.DateTimeFormat(locale, {
      hour: twelve ? "numeric" : "2-digit",
      minute: "2-digit",
    });
    clocks.set(locale, clock);
  }
  return clock.format(date);
};

/**
 * A calendar day ("2026-10-12", or the start of an RFC 3339 time) as a medium date in `locale`.
 * A malformed or out-of-range day comes back as written, never rolled over to another date.
 */
export const formatIsoDay = (iso: string | null, locale: string): string => {
  const text = (iso ?? "").slice(0, 10);
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(text);
  if (!match) return text;
  const [year, month, day] = [
    Number(match[1]),
    Number(match[2]),
    Number(match[3]),
  ];
  const date = new Date(year, month - 1, day);
  date.setFullYear(year);
  if (date.getMonth() !== month - 1 || date.getDate() !== day) return text;
  return new Intl.DateTimeFormat(locale, { dateStyle: "medium" }).format(date);
};
