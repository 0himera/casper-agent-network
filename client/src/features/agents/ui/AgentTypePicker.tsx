import { Cloud, Monitor } from "lucide-react";
import type { AgentExecutionMode } from "@/entities/agent/types/types";
import styles from "./Register.module.css";

interface AgentTypePickerProps {
  type: AgentExecutionMode;
  onChange: (t: AgentExecutionMode) => void;
  endpoint: string;
  apiKey: string;
  model: string;
  onEndpointChange: (v: string) => void;
  onApiKeyChange: (v: string) => void;
  onModelChange: (v: string) => void;
}

export function AgentTypePicker(p: AgentTypePickerProps) {
  return (
    <div className={styles.field}>
      <label className={styles.label}>Agent Type</label>
      <div className={styles.typeOptions}>
        <div className={`${styles.typeOption} ${p.type === "hosted" ? styles.typeSelected : ""}`} onClick={() => p.onChange("hosted")}>
          <div className={styles.typeHeader}><Cloud size={14} /> Hosted Agent (API Endpoint)</div>
          <div className={styles.typeDescription}>Agent runs via cloud API. Provide endpoint, API key, model.</div>
          {p.type === "hosted" && (
            <div className={styles.hostedFields}>
              <input className={styles.input} placeholder="Endpoint URL" value={p.endpoint} onChange={(e) => p.onEndpointChange(e.target.value)} />
              <input className={styles.input} placeholder="API Key" value={p.apiKey} onChange={(e) => p.onApiKeyChange(e.target.value)} type="password" />
              <input className={styles.input} placeholder="Model ID" value={p.model} onChange={(e) => p.onModelChange(e.target.value)} />
            </div>
          )}
        </div>
        <div className={`${styles.typeOption} ${p.type === "autonomous" ? styles.typeSelected : ""}`} onClick={() => p.onChange("autonomous")}>
          <div className={styles.typeHeader}><Monitor size={14} /> Autonomous Agent (Self-hosted)</div>
          <div className={styles.typeDescription}>Bot runs 24/7 on your server with its own PEM wallet key.</div>
        </div>
      </div>
    </div>
  );
}
