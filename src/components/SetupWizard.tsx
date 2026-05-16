import { useState, useEffect } from "react";
import * as api from "../tauri-api";
import { FPS_OPTIONS, QUALITY_OPTIONS, CLIP_MODE_OPTIONS } from "../types";
import type { WindowEntry } from "../types";
import LogoPlaceholder from "./LogoPlaceholder";
import HotkeyRecorder from "./HotkeyRecorder";

interface Props {
  onSetupComplete: () => void;
}

export default function SetupWizard({ onSetupComplete }: Props) {
  const [outputFolder, setOutputFolder] = useState("~/.penguclip/clips");
  const [recordingFps, setRecordingFps] = useState("fps60");
  const [videoQuality, setVideoQuality] = useState("medium");
  const [clipDuration, setClipDuration] = useState(30);
  const [maxRecording, setMaxRecording] = useState(0);
  const [hotkey, setHotkey] = useState("ControlLeft+KeyR");
  const [clipMode, setClipMode] = useState("anything");
  const [appFilters, setAppFilters] = useState<string[]>([]);
  const [windows, setWindows] = useState<WindowEntry[]>([]);
  const [loadingWindows, setLoadingWindows] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    if (clipMode === "apps") {
      loadWindows();
    }
  }, [clipMode]);

  async function loadWindows() {
    setLoadingWindows(true);
    try {
      const wins = await api.listWindows();
      setWindows(wins);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoadingWindows(false);
    }
  }

  function toggleAppFilter(cls: string) {
    setAppFilters((prev) =>
      prev.includes(cls) ? prev.filter((c) => c !== cls) : [...prev, cls]
    );
  }

  async function handlePickFolder() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({ directory: true, title: "Select Clip Output Folder" });
      if (selected) setOutputFolder(selected);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleSave() {
    setSaving(true);
    setError("");
    try {
      await api.saveConfig({
        outputFolder,
        recordingFps,
        videoQuality,
        clipDurationSecs: clipDuration,
        maxRecordingSecs: maxRecording,
        hotkey,
        clipMode,
        appFilters,
      });
      onSetupComplete();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="setup-wizard">
      <LogoPlaceholder />
      <h1>Welcome to Penguclip</h1>
      <p className="subtitle">High-performance background clipping for Linux</p>

      <div className="setup-form">
        <label>
          <span>Output Folder</span>
          <div style={{ display: "flex", gap: 8 }}>
            <input type="text" value={outputFolder} onChange={(e) => setOutputFolder(e.target.value)} style={{ flex: 1 }} />
            <button className="btn-secondary" type="button" onClick={handlePickFolder}>Browse</button>
          </div>
        </label>

        <label>
          <span>Clip Mode</span>
          <select value={clipMode} onChange={(e) => setClipMode(e.target.value)}>
            {CLIP_MODE_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>{opt.label}</option>
            ))}
          </select>
          <small>What should Penguclip capture?</small>
        </label>

        {clipMode === "apps" && (
          <label>
            <span>Select Apps to Capture</span>
            <div style={{ maxHeight: 150, overflowY: "auto", background: "var(--bg-input)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", padding: 8 }}>
              {loadingWindows ? (
                <span style={{ color: "var(--text-muted)", fontSize: 12 }}>Loading windows... (install xdotool: sudo pacman -S xdotool)</span>
              ) : windows.length === 0 ? (
                <span style={{ color: "var(--text-muted)", fontSize: 12 }}>No windows found. Install xdotool or wmctrl.</span>
              ) : (
                windows.map((w) => (
                  <label key={w.id} style={{ display: "flex", alignItems: "center", gap: 8, padding: "4px 0", cursor: "pointer", fontSize: 12 }}>
                    <input type="checkbox" checked={appFilters.includes(w.class)} onChange={() => toggleAppFilter(w.class)} />
                    <span style={{ color: "var(--text-primary)" }}>{w.title || w.class}</span>
                    <span style={{ color: "var(--text-muted)", fontSize: 10 }}>({w.class})</span>
                  </label>
                ))
              )}
            </div>
            <small>Only apps with checked boxes will be captured</small>
          </label>
        )}

        <label>
          <span>Recording FPS</span>
          <select value={recordingFps} onChange={(e) => setRecordingFps(e.target.value)}>
            {FPS_OPTIONS.map((opt) => (<option key={opt.value} value={opt.value}>{opt.label}</option>))}
          </select>
        </label>

        <label>
          <span>Video Quality</span>
          <select value={videoQuality} onChange={(e) => setVideoQuality(e.target.value)}>
            {QUALITY_OPTIONS.map((opt) => (<option key={opt.value} value={opt.value}>{opt.label}</option>))}
          </select>
        </label>

        <label>
          <span>Clip Duration (seconds)</span>
          <input type="number" min={5} max={300} value={clipDuration} onChange={(e) => setClipDuration(parseInt(e.target.value) || 30)} />
          <small>Seconds saved when hotkey pressed (5–300)</small>
        </label>

        <label>
          <span>Max Recording Duration (0 = unlimited)</span>
          <input type="number" min={0} max={36000} value={maxRecording} onChange={(e) => setMaxRecording(parseInt(e.target.value) || 0)} />
          <small>Auto-stop after N seconds. 0 = record until manually stopped</small>
        </label>

        <label>
          <span>Global Hotkey</span>
          <HotkeyRecorder value={hotkey} onChange={setHotkey} />
          <small>Click the input then press your combo</small>
        </label>

        {error && <div className="error-message">{error}</div>}

        <button className="btn-primary" onClick={handleSave} disabled={saving}>
          {saving ? "Saving..." : "Save & Continue"}
        </button>
      </div>
    </div>
  );
}
