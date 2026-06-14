"use client";

import { useState } from "react";
import { Wrench } from "lucide-react";
import type { AgentSkill, AgentExecutionMode } from "@/entities/agent/types/types";
import { SkillsPicker } from "@/features/agents/ui/SkillsPicker";
import { AgentTypePicker } from "@/features/agents/ui/AgentTypePicker";
import styles from "@/features/agents/ui/Register.module.css";

export default function RegisterPage() {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [skills, setSkills] = useState<AgentSkill[]>([]);
  const [agentType, setAgentType] = useState<AgentExecutionMode>("hosted");
  const [endpoint, setEndpoint] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [model, setModel] = useState("");

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    alert("Agent registered! (simulated)");
  };

  return (
    <div className={styles.page}>
      <h1 className={styles.title}><Wrench size={20} /> Register Bot</h1>
      <form className={styles.form} onSubmit={handleSubmit}>
        <div className={styles.field}>
          <label className={styles.label}>Agent Name</label>
          <input className={styles.input} value={name} onChange={(e) => setName(e.target.value)} placeholder="My DeFi Bot" />
        </div>
        <div className={styles.field}>
          <label className={styles.label}>Description</label>
          <textarea className={styles.textarea} value={description} onChange={(e) => setDescription(e.target.value)} placeholder="What does your agent do?" />
        </div>
        <SkillsPicker selected={skills} onChange={setSkills} />
        <AgentTypePicker type={agentType} onChange={setAgentType} endpoint={endpoint} apiKey={apiKey} model={model} onEndpointChange={setEndpoint} onApiKeyChange={setApiKey} onModelChange={setModel} />
        <button type="submit" className={styles.submitButton}>Sign &amp; Register Agent On-Chain</button>
      </form>
    </div>
  );
}
