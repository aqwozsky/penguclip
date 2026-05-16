import { useState, useEffect, useCallback } from "react";
import * as api from "../tauri-api";
import type { ClipEntry } from "../tauri-api";

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

/** Browse, play, trim, and delete saved clips. */
export default function ClipsViewer() {
  const [clips, setClips] = useState<ClipEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [trimPath, setTrimPath] = useState<string | null>(null);
  const [trimStart, setTrimStart] = useState("0");
  const [trimEnd, setTrimEnd] = useState("10");
  const [trimming, setTrimming] = useState(false);

  const loadClips = useCallback(async () => {
    setLoading(true);
    try {
      const result = await api.listClips();
      setClips(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadClips();
  }, [loadClips]);

  async function handlePlay(path: string) {
    try {
      await api.openFile(path);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleDelete(path: string) {
    try {
      await api.deleteClip(path);
      setClips((prev) => prev.filter((c) => c.path !== path));
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleTrim() {
    if (!trimPath) return;
    setTrimming(true);
    setError("");
    try {
      await api.trimClip(
        trimPath,
        parseFloat(trimStart),
        parseFloat(trimEnd)
      );
      setTrimPath(null);
      await loadClips();
    } catch (e) {
      setError(String(e));
    } finally {
      setTrimming(false);
    }
  }

  if (loading) {
    return <div className="no-clips">Loading clips...</div>;
  }

  return (
    <div className="clips-viewer">
      {error && <div className="error-message" style={{ marginBottom: 12 }}>{error}</div>}

      {/* Trim Dialog */}
      {trimPath && (
        <div className="info-section" style={{ marginBottom: 16 }}>
          <h3 style={{ marginBottom: 12 }}>Trim Clip</h3>
          <p style={{ fontSize: 12, color: "var(--text-muted)", marginBottom: 12, wordBreak: "break-all" }}>
            {trimPath.split("/").pop()}
          </p>
          <div style={{ display: "flex", gap: 12, marginBottom: 12 }}>
            <label style={{ flex: 1 }}>
              <span style={{ fontSize: 11, color: "var(--text-muted)", display: "block", marginBottom: 4 }}>Start (seconds)</span>
              <input
                type="number"
                min="0"
                step="0.5"
                value={trimStart}
                onChange={(e) => setTrimStart(e.target.value)}
                style={{
                  width: "100%",
                  background: "var(--bg-input)",
                  border: "1px solid var(--border-color)",
                  borderRadius: "var(--radius-sm)",
                  color: "var(--text-primary)",
                  padding: "8px 12px",
                  fontSize: 14,
                }}
              />
            </label>
            <label style={{ flex: 1 }}>
              <span style={{ fontSize: 11, color: "var(--text-muted)", display: "block", marginBottom: 4 }}>End (seconds)</span>
              <input
                type="number"
                min="0.5"
                step="0.5"
                value={trimEnd}
                onChange={(e) => setTrimEnd(e.target.value)}
                style={{
                  width: "100%",
                  background: "var(--bg-input)",
                  border: "1px solid var(--border-color)",
                  borderRadius: "var(--radius-sm)",
                  color: "var(--text-primary)",
                  padding: "8px 12px",
                  fontSize: 14,
                }}
              />
            </label>
          </div>
          <div style={{ display: "flex", gap: 8 }}>
            <button className="btn-primary" onClick={handleTrim} disabled={trimming} style={{ padding: "8px 16px", fontSize: 13 }}>
              {trimming ? "Trimming..." : "Trim"}
            </button>
            <button className="btn-secondary" onClick={() => setTrimPath(null)} style={{ padding: "8px 16px", fontSize: 13 }}>
              Cancel
            </button>
          </div>
        </div>
      )}

      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
        <span style={{ fontSize: 12, color: "var(--text-muted)" }}>
          {clips.length} clip{clips.length !== 1 ? "s" : ""}
        </span>
        <button className="btn-icon" onClick={loadClips} title="Refresh">
          ↻ Refresh
        </button>
      </div>

      {clips.length === 0 ? (
        <div className="no-clips">
          No clips yet. Start recording and press the hotkey!
        </div>
      ) : (
        <div className="clips-list">
          {clips.map((clip) => (
            <div key={clip.path} className="clip-item">
              <div className="clip-info">
                <span className="clip-name">{clip.name}</span>
                <span className="clip-meta">
                  <span>{formatSize(clip.size_bytes)}</span>
                  <span>{clip.modified}</span>
                </span>
              </div>
              <div className="clip-actions">
                <button className="btn-icon" onClick={() => handlePlay(clip.path)} title="Play">
                  ▶
                </button>
                <button className="btn-icon" onClick={() => { setTrimPath(clip.path); setTrimStart("0"); setTrimEnd("10"); }} title="Trim">
                  ✂
                </button>
                <button className="btn-icon danger" onClick={() => handleDelete(clip.path)} title="Delete">
                  ✕
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
