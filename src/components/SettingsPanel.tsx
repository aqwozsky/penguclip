import { useState, useEffect } from "react";
import * as api from "../tauri-api";
import type { AppConfig, WindowEntry } from "../types";
import { FPS_OPTIONS, QUALITY_OPTIONS, CLIP_MODE_OPTIONS } from "../types";
import HotkeyRecorder from "./HotkeyRecorder";

export default function SettingsPanel() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [outputFolder, setOutputFolder] = useState("");
  const [recordingFps, setRecordingFps] = useState("fps60");
  const [videoQuality, setVideoQuality] = useState("medium");
  const [clipDuration, setClipDuration] = useState(30);
  const [maxRecording, setMaxRecording] = useState(0);
  const [hotkey, setHotkey] = useState("");
  const [clipMode, setClipMode] = useState("anything");
  const [appFilters, setAppFilters] = useState<string[]>([]);
  const [windows, setWindows] = useState<WindowEntry[]>([]);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");

  useEffect(() => {
    api.getConfig().then((cfg) => {
      if (cfg) {
        setConfig(cfg);
        setOutputFolder(cfg.outputFolder);
        setRecordingFps(cfg.recordingFps);
        setVideoQuality(cfg.videoQuality);
        setClipDuration(cfg.clipDurationSecs);
        setMaxRecording(cfg.maxRecordingSecs || 0);
        setHotkey(cfg.hotkey);
        setClipMode(cfg.clipMode || "anything");
        setAppFilters(cfg.appFilters || []);
      }
    });
  }, []);

  useEffect(() => {
    if (clipMode === "apps") {
      api.listWindows().then(setWindows).catch(() => {});
    }
  }, [clipMode]);

  async function handlePickFolder() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({ directory: true, title: "Select Clip Output Folder" });
      if (selected) setOutputFolder(selected);
    } catch (e) { setError(String(e)); }
  }

  function toggleAppFilter(cls: string) {
    setAppFilters((prev) => prev.includes(cls) ? prev.filter((c) => c !== cls) : [...prev, cls]);
  }

  async function handleSave() {
    setSaving(true);
    setError(""); setMessage("");
    try {
      const updated = await api.updateSettings({
        outputFolder: outputFolder !== config?.outputFolder ? outputFolder : undefined,
        recordingFps: recordingFps !== config?.recordingFps ? recordingFps : undefined,
        videoQuality: videoQuality !== config?.videoQuality ? videoQuality : undefined,
        clipDurationSecs: clipDuration !== config?.clipDurationSecs ? clipDuration : undefined,
        maxRecordingSecs: maxRecording !== (config?.maxRecordingSecs || 0) ? maxRecording : undefined,
        hotkey: hotkey !== config?.hotkey ? hotkey : undefined,
        clipMode: clipMode !== config?.clipMode ? clipMode : undefined,
        appFilters: JSON.stringify(appFilters) !== JSON.stringify(config?.appFilters) ? appFilters : undefined,
      });
      setConfig(updated);
      setMessage("Settings saved!");
      setTimeout(() => setMessage(""), 3000);
    } catch (e) {
      setError(String(e));
    } finally { setSaving(false); }
  }

  if (!config) return <div className="no-clips">Loading...</div>;

  return (
    <div className="settings-panel">
      {error && <div className="error-message">{error}</div>}
      {message && <div className="clip-saved-notice" style={{ borderColor: "var(--accent)", color: "var(--accent)" }}>{message}</div>}

      <div className="setting-group">
        <label><span>Output Folder</span>
          <div style={{ display: "flex", gap: 8 }}>
            <input type="text" value={outputFolder} onChange={(e) => setOutputFolder(e.target.value)} style={{ flex: 1 }} />
            <button className="btn-secondary" onClick={handlePickFolder}>Browse</button>
          </div>
        </label>
      </div>

      <div className="setting-group">
        <label><span>Clip Mode</span>
          <select value={clipMode} onChange={(e) => setClipMode(e.target.value)}>
            {CLIP_MODE_OPTIONS.map((o) => (<option key={o.value} value={o.value}>{o.label}</option>))}
          </select>
        </label>
      </div>

      {clipMode === "apps" && (
        <div className="setting-group">
          <label><span>Apps to Capture</span>
            <div style={{ maxHeight: 150, overflowY: "auto", background: "var(--bg-input)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", padding: 8 }}>
              {windows.length === 0 ? (
                <span style={{ color: "var(--text-muted)", fontSize: 12 }}>Install xdotool to list windows</span>
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
          </label>
        </div>
      )}

      <div className="setting-group"><label><span>Recording FPS</span><select value={recordingFps} onChange={(e) => setRecordingFps(e.target.value)}>{FPS_OPTIONS.map((o) => (<option key={o.value} value={o.value}>{o.label}</option>))}</select></label></div>
      <div className="setting-group"><label><span>Video Quality</span><select value={videoQuality} onChange={(e) => setVideoQuality(e.target.value)}>{QUALITY_OPTIONS.map((o) => (<option key={o.value} value={o.value}>{o.label}</option>))}</select></label></div>
      <div className="setting-group"><label><span>Clip Duration (s)</span><input type="number" min={5} max={300} value={clipDuration} onChange={(e) => setClipDuration(parseInt(e.target.value) || 30)} /></label></div>
      <div className="setting-group"><label><span>Max Recording (s, 0=∞)</span><input type="number" min={0} max={36000} value={maxRecording} onChange={(e) => setMaxRecording(parseInt(e.target.value) || 0)} /></label></div>

      <div className="setting-group">
        <label><span>Global Hotkey</span><HotkeyRecorder value={hotkey} onChange={setHotkey} /></label>
      </div>

      <div className="settings-actions">
        <button className="btn-primary" onClick={handleSave} disabled={saving}>{saving ? "Saving..." : "Save Settings"}</button>
        <button className="btn-secondary" onClick={() => {
          if (!config) return;
          setOutputFolder(config.outputFolder); setRecordingFps(config.recordingFps);
          setVideoQuality(config.videoQuality); setClipDuration(config.clipDurationSecs);
          setMaxRecording(config.maxRecordingSecs || 0); setHotkey(config.hotkey);
          setClipMode(config.clipMode || "anything"); setAppFilters(config.appFilters || []);
        }}>Reset</button>
      </div>
    </div>
  );
}
