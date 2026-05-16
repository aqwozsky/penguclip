export interface AppConfig {
  outputFolder: string;
  recordingFps: "fps30" | "fps60" | "fps120";
  videoQuality: "low" | "medium" | "high";
  clipDurationSecs: number;
  maxRecordingSecs: number;
  hotkey: string;
  clipMode: "anything" | "games" | "apps";
  appFilters: string[];
  setupComplete: boolean;
}

export interface AppStatus {
  recording: boolean;
  encoder: string | null;
  hotkey: string;
  outputFolder: string;
  clipDurationSecs: number;
  clip_mode: string;
  maxRecordingSecs: number;
  setupComplete: boolean;
}

export interface WindowEntry {
  id: string;
  title: string;
  class: string;
}

export const FPS_OPTIONS = [
  { value: "fps30", label: "30 FPS" },
  { value: "fps60", label: "60 FPS" },
  { value: "fps120", label: "120 FPS" },
] as const;

export const QUALITY_OPTIONS = [
  { value: "low", label: "Low (smaller files)" },
  { value: "medium", label: "Medium (balanced)" },
  { value: "high", label: "High (best quality)" },
] as const;

export const CLIP_MODE_OPTIONS = [
  { value: "anything", label: "Clip Anything — entire screen" },
  { value: "games", label: "Clip Games Only — auto-detect" },
  { value: "apps", label: "Clip Specific Apps — choose windows" },
] as const;
