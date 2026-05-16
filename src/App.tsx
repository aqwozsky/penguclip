import { useState, useEffect } from "react";
import * as api from "./tauri-api";
import SetupWizard from "./components/SetupWizard";
import MainDashboard from "./components/MainDashboard";
import "./App.css";

/**
 * App root — routes between SetupWizard (first launch) and
 * MainDashboard (subsequent launches) based on config.setupComplete.
 */
export default function App() {
  const [setupComplete, setSetupComplete] = useState<boolean | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    api
      .getConfig()
      .then((config) => {
        setSetupComplete(config?.setupComplete ?? false);
      })
      .catch((err) => {
        console.error("Failed to load config:", err);
        setSetupComplete(false);
      })
      .finally(() => setLoading(false));
  }, []);

  if (loading) {
    return (
      <div className="loading-screen">
        <div className="spinner" />
        <p>Penguclip</p>
      </div>
    );
  }

  if (!setupComplete) {
    return (
      <SetupWizard
        onSetupComplete={() => setSetupComplete(true)}
      />
    );
  }

  return <MainDashboard />;
}
