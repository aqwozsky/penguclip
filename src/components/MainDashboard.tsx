import { useState, useEffect } from "react";
import * as api from "../tauri-api";
import type { AppStatus } from "../types";
import LogoPlaceholder from "./LogoPlaceholder";
import StatusBar from "./StatusBar";
import ClipsViewer from "./ClipsViewer";
import SettingsPanel from "./SettingsPanel";

type Page = "dashboard" | "clips" | "settings";

/** Main app with left sidebar navigation. */
export default function MainDashboard() {
  const [page, setPage] = useState<Page>("dashboard");
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [recording, setRecording] = useState(false);
  const [encoder, setEncoder] = useState<string | null>(null);
  const [error, setError] = useState("");
  const [lastClip, setLastClip] = useState("");

  useEffect(() => {
    api.getStatus().then(setStatus).catch(console.error);
    api.detectEncoder().then(setEncoder).catch(console.error);
  }, []);

  async function handleToggleRecording() {
    setError("");
    try {
      if (recording) {
        await api.stopRecording();
        setRecording(false);
      } else {
        await api.startRecording();
        setRecording(true);
      }
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleManualSave() {
    setError("");
    try {
      const path = await api.saveClip();
      setLastClip(path);
      // Auto-switch to clips tab to show the new clip
      setPage("clips");
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleOpenFolder() {
    try {
      await api.openOutputFolder();
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div className="app-layout">
      {/* ── Left Sidebar ── */}
      <nav className="sidebar">
        <div className="sidebar-logo">
          <LogoPlaceholder />
        </div>

        <div className="sidebar-nav">
          <button
            className={`sidebar-item ${page === "dashboard" ? "active" : ""}`}
            onClick={() => setPage("dashboard")}
          >
            <span className="sidebar-icon">◉</span>
            <span>Dashboard</span>
          </button>

          <button
            className={`sidebar-item ${page === "clips" ? "active" : ""}`}
            onClick={() => setPage("clips")}
          >
            <span className="sidebar-icon">▶</span>
            <span>Clips</span>
          </button>

          <button
            className={`sidebar-item ${page === "settings" ? "active" : ""}`}
            onClick={() => setPage("settings")}
          >
            <span className="sidebar-icon">⚙</span>
            <span>Settings</span>
          </button>

          <div className="sidebar-divider" />

          <button className="sidebar-item" onClick={handleOpenFolder}>
            <span className="sidebar-icon">📂</span>
            <span>Open Folder</span>
          </button>
        </div>

        <div className="sidebar-footer">
          <StatusBar recording={recording} encoder={encoder} />
        </div>
      </nav>

      {/* ── Main Content ── */}
      <main className="content">
        {error && <div className="error-message">{error}</div>}

        {lastClip && page === "dashboard" && (
          <div className="clip-saved-notice">
            ✓ Clip saved: <code>{lastClip}</code>
          </div>
        )}

        {page === "dashboard" && (
          <>
            <h2 className="page-title">Dashboard</h2>

            <div className="controls">
              <button
                className={`btn-record ${recording ? "recording" : ""}`}
                onClick={handleToggleRecording}
              >
                {recording ? "■ Stop Recording" : "● Start Recording"}
              </button>
              <button
                className="btn-secondary"
                onClick={handleManualSave}
                disabled={!recording}
              >
                Save Clip (Manual)
              </button>
            </div>

            {status && (
              <div className="info-section">
                <h3>Current Settings</h3>
                <div className="settings-grid">
                  <div className="setting-item">
                    <span className="setting-label">Output</span>
                    <span className="setting-value">
                      {status.outputFolder}
                    </span>
                  </div>
                  <div className="setting-item">
                    <span className="setting-label">Clip Duration</span>
                    <span className="setting-value">
                      {status.clipDurationSecs}s
                    </span>
                  </div>
                  <div className="setting-item">
                    <span className="setting-label">Hotkey</span>
                    <span className="setting-value">
                      <kbd>{status.hotkey.replace("ControlLeft", "Ctrl").replace("Key", "")}</kbd>
                    </span>
                  </div>
                  <div className="setting-item">
                    <span className="setting-label">Encoder</span>
                    <span className="setting-value">
                      {encoder || "Detecting..."}
                    </span>
                  </div>
                </div>
              </div>
            )}

            <div className="hotkey-hint">
              <p>
                Press{" "}
                <kbd>
                  {(status?.hotkey || "Ctrl+R")
                    .replace("ControlLeft", "Ctrl")
                    .replace("Key", "")}
                </kbd>{" "}
                to save the last {status?.clipDurationSecs || 30}s
              </p>
              <p className="wayland-note">
                Wayland: bind a system shortcut to send SIGUSR1
              </p>
            </div>
          </>
        )}

        {page === "clips" && (
          <>
            <h2 className="page-title">Clips</h2>
            <ClipsViewer />
          </>
        )}

        {page === "settings" && (
          <>
            <h2 className="page-title">Settings</h2>
            <SettingsPanel />
          </>
        )}
      </main>
    </div>
  );
}
