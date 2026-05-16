/** Visual status indicator — pulsing red dot when recording. */
interface Props {
  recording: boolean;
  encoder: string | null;
}

export default function StatusBar({ recording, encoder }: Props) {
  return (
    <div className={`status-bar ${recording ? "active" : ""}`}>
      <span className={`status-dot ${recording ? "pulse" : ""}`} />
      <span className="status-text">
        {recording ? "Recording" : "Idle"}
      </span>
      {encoder && (
        <span className="status-encoder">
          &nbsp;· {encoder}
        </span>
      )}
    </div>
  );
}
