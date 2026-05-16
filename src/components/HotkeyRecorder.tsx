import { useState, useEffect, useCallback } from "react";

interface Props {
  value: string;
  onChange: (hotkey: string) => void;
}

/**
 * Hotkey recorder — click to start recording, press any key combo,
 * and it auto-detects Ctrl/Alt/Shift modifiers + the main key.
 */
export default function HotkeyRecorder({ value, onChange }: Props) {
  const [listening, setListening] = useState(false);
  const [displayKey, setDisplayKey] = useState("");

  // Convert stored format like "ControlLeft+KeyR" to display "Ctrl+R"
  useEffect(() => {
    const display = value
      .replace("ControlLeft", "Ctrl")
      .replace("ControlRight", "Ctrl")
      .replace("AltGr", "Alt")
      .replace("ShiftLeft", "Shift")
      .replace("ShiftRight", "Shift")
      .replace("Key", "");
    setDisplayKey(display);
  }, [value]);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (!listening) return;
      e.preventDefault();
      e.stopPropagation();

      // Ignore modifier-only presses
      if (["Control", "Alt", "Shift", "Meta"].includes(e.key)) return;

      const parts: string[] = [];
      if (e.ctrlKey) parts.push("ControlLeft");
      if (e.altKey) parts.push("Alt");
      if (e.shiftKey) parts.push("ShiftLeft");

      // Map the key to rdev format
      const keyName = mapKeyToRdev(e.key, e.code);
      parts.push(keyName);

      const combo = parts.join("+");
      onChange(combo);
      setListening(false);
    },
    [listening, onChange]
  );

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown, true);
    return () => window.removeEventListener("keydown", handleKeyDown, true);
  }, [handleKeyDown]);

  return (
    <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
      <input
        type="text"
        value={listening ? "Press any key combo..." : displayKey}
        readOnly
        style={{
          flex: 1,
          background: listening ? "rgba(255,255,255,0.1)" : "var(--bg-input)",
          border: listening ? "2px solid var(--accent)" : "1px solid var(--border-color)",
          borderRadius: "var(--radius-sm)",
          color: listening ? "var(--accent)" : "var(--text-primary)",
          padding: "10px 12px",
          fontSize: 14,
          fontFamily: "monospace",
          transition: "all 150ms ease",
          outline: "none",
        }}
        onClick={() => setListening(true)}
      />
      <button
        className={listening ? "btn-record recording" : "btn-secondary"}
        style={{ flexShrink: 0 }}
        onClick={() => setListening(!listening)}
      >
        {listening ? "Listening..." : "Record"}
      </button>
    </div>
  );
}

/** Map DOM KeyboardEvent to rdev Key format. */
function mapKeyToRdev(key: string, code: string): string {
  // Function keys
  if (code.startsWith("F") && code.length <= 3) return code;
  // Numpad
  if (code.startsWith("Numpad")) return code.replace("Numpad", "Num");
  // Special keys
  const codeMap: Record<string, string> = {
    Space: "Space",
    Escape: "Escape",
    Tab: "Tab",
    Backspace: "BackSpace",
    Enter: "Enter",
    ArrowUp: "UpArrow",
    ArrowDown: "DownArrow",
    ArrowLeft: "LeftArrow",
    ArrowRight: "RightArrow",
    Insert: "Insert",
    Delete: "Delete",
    Home: "Home",
    End: "End",
    PageUp: "PageUp",
    PageDown: "PageDown",
    PrintScreen: "PrintScreen",
  };
  if (codeMap[code]) return codeMap[code];

  // Single character keys → "KeyX" format
  if (key.length === 1) return `Key${key.toUpperCase()}`;

  return `Key${key.toUpperCase()}`;
}
