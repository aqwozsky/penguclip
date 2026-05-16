import { invoke } from "@tauri-apps/api/core";
import type { AppConfig, AppStatus, WindowEntry } from "./types";

// ─── Config ────────────────────────────────────────────────────

export async function getConfig(): Promise<AppConfig | null> {
  return invoke<AppConfig | null>("get_config");
}

export async function saveConfig(params: {
  outputFolder: string;
  recordingFps: string;
  videoQuality: string;
  clipDurationSecs: number;
  maxRecordingSecs: number;
  hotkey: string;
  clipMode: string;
  appFilters: string[];
}): Promise<AppConfig> {
  return invoke<AppConfig>("save_config", {
    outputFolder: params.outputFolder,
    recordingFps: params.recordingFps,
    videoQuality: params.videoQuality,
    clipDurationSecs: params.clipDurationSecs,
    maxRecordingSecs: params.maxRecordingSecs,
    hotkey: params.hotkey,
    clipMode: params.clipMode,
    appFilters: params.appFilters,
  });
}

// ─── Encoder ───────────────────────────────────────────────────

export async function detectEncoder(): Promise<string> {
  return invoke<string>("detect_encoder");
}

// ─── Recording ─────────────────────────────────────────────────

export async function startRecording(): Promise<string> {
  return invoke<string>("start_recording");
}

export async function stopRecording(): Promise<string> {
  return invoke<string>("stop_recording");
}

export async function saveClip(): Promise<string> {
  return invoke<string>("save_clip");
}

export async function getStatus(): Promise<AppStatus> {
  return invoke<AppStatus>("get_status");
}

// ─── Clips ─────────────────────────────────────────────────────

export interface ClipEntry {
  name: string;
  path: string;
  size_bytes: number;
  modified: string;
}

export async function listClips(): Promise<ClipEntry[]> {
  return invoke<ClipEntry[]>("list_clips");
}

export async function deleteClip(path: string): Promise<void> {
  return invoke<void>("delete_clip", { path });
}

export async function trimClip(
  path: string,
  startSecs: number,
  endSecs: number
): Promise<string> {
  return invoke<string>("trim_clip", { path, startSecs, endSecs });
}

export async function openFile(path: string): Promise<void> {
  return invoke<void>("open_file", { path });
}

// ─── Settings ──────────────────────────────────────────────────

export async function updateSettings(params: {
  outputFolder?: string;
  recordingFps?: string;
  videoQuality?: string;
  clipDurationSecs?: number;
  maxRecordingSecs?: number;
  hotkey?: string;
  clipMode?: string;
  appFilters?: string[];
}): Promise<AppConfig> {
  return invoke<AppConfig>("update_settings", {
    outputFolder: params.outputFolder ?? null,
    recordingFps: params.recordingFps ?? null,
    videoQuality: params.videoQuality ?? null,
    clipDurationSecs: params.clipDurationSecs ?? null,
    maxRecordingSecs: params.maxRecordingSecs ?? null,
    hotkey: params.hotkey ?? null,
    clipMode: params.clipMode ?? null,
    appFilters: params.appFilters ?? null,
  });
}

// ─── Window Detection ──────────────────────────────────────────

export async function listWindows(): Promise<WindowEntry[]> {
  return invoke<WindowEntry[]>("list_windows");
}

export async function detectGame(): Promise<boolean> {
  return invoke<boolean>("detect_game");
}

// ─── Misc ──────────────────────────────────────────────────────

export async function openOutputFolder(): Promise<void> {
  return invoke<void>("open_output_folder");
}
