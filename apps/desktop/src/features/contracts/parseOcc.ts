/** OCC / compact broker option symbols. Money is strike_minor (cents, scale 2). */

export type OccRight = "call" | "put";

export type ParsedOcc = {
  underlying: string;
  expiryIso: string;
  right: OccRight;
  /** Strike in cents (scale 2). $20.70 → 2070. */
  strikeMinor: number;
  putCall: "C" | "P";
  raw: string;
};

function expiryFromYymmdd(yymmdd: string): string | null {
  if (!/^\d{6}$/.test(yymmdd)) return null;
  const yy = Number(yymmdd.slice(0, 2));
  const mm = Number(yymmdd.slice(2, 4));
  const dd = Number(yymmdd.slice(4, 6));
  if (mm < 1 || mm > 12 || dd < 1 || dd > 31) return null;
  const year = 2000 + yy;
  const iso = `${year}-${String(mm).padStart(2, "0")}-${String(dd).padStart(2, "0")}`;
  const probe = Date.parse(`${iso}T00:00:00`);
  if (!Number.isFinite(probe)) return null;
  const d = new Date(probe);
  if (d.getFullYear() !== year || d.getMonth() + 1 !== mm || d.getDate() !== dd) {
    return null;
  }
  return iso;
}

function dollarsToStrikeMinor(raw: string): number | null {
  const n = Number(raw);
  if (!Number.isFinite(n) || n < 0) return null;
  return Math.round(n * 100);
}

/**
 * Accept OSI 21-char and compact broker paste.
 * Fixtures: `.TSLL1270115C20.7` / `TSLL270115C20.7` → TSLL 2027-01-15 C 2070;
 * `.AAPL251219P250` → AAPL 2025-12-19 P 25000;
 * `AAPL  250117C00150000` → AAPL 2025-01-17 C 15000.
 */
export function parseOcc(symbol: string): ParsedOcc | null {
  const upper = symbol.trim().toUpperCase();
  if (!upper) return null;

  // OSI: root padded to 6 + YYMMDD + C/P + 8-digit strike×1000.
  const spaced = upper.length === 21 ? upper : null;
  if (spaced && /^[A-Z0-9.\-]{6}\d{6}[CP]\d{8}$/.test(spaced)) {
    const root = spaced.slice(0, 6).trim();
    const expiryIso = expiryFromYymmdd(spaced.slice(6, 12));
    const cp = spaced.slice(12, 13) as "C" | "P";
    const strikeRaw = Number(spaced.slice(13, 21));
    if (!root || !expiryIso || !Number.isFinite(strikeRaw)) return null;
    // strike×1000 dollars → cents: /1000 * 100 = /10
    const strikeMinor = Math.round(strikeRaw / 10);
    return {
      underlying: root,
      expiryIso,
      right: cp === "P" ? "put" : "call",
      strikeMinor,
      putCall: cp,
      raw: spaced.replace(/\s+/g, ""),
    };
  }

  const compact = upper.replace(/\s+/g, "").replace(/^\./, "");
  // Compact: ROOT + (YYMMDD | 1+YYMMDD) + C|P + decimal strike dollars.
  const m = compact.match(/^([A-Z]{1,6})(\d{6}|\d{7})([CP])(\d+(?:\.\d+)?)$/);
  if (!m) return null;
  const underlying = m[1];
  let yymmdd = m[2];
  if (yymmdd.length === 7) {
    if (yymmdd[0] !== "1") return null;
    yymmdd = yymmdd.slice(1);
  }
  const expiryIso = expiryFromYymmdd(yymmdd);
  if (!expiryIso) return null;
  const cp = m[3] as "C" | "P";
  const strikeMinor = dollarsToStrikeMinor(m[4]);
  if (strikeMinor == null) return null;
  return {
    underlying,
    expiryIso,
    right: cp === "P" ? "put" : "call",
    strikeMinor,
    putCall: cp,
    raw: compact,
  };
}
