export interface TerminalLogItem {
  id: string;
  timestamp: string;
  type: "info" | "success" | "warn" | "error" | "system";
  message: string;
}

export interface SpecCardItem {
  id: string;
  title: string;
  subtitle: string;
  description: string;
  codeSnippet: string;
  language: string;
}

export interface MetricItemData {
  id: string;
  label: string;
  value: number;
  suffix: string;
}

export interface RoadmapItemData {
  id: string;
  phase: string;
  title: string;
  date: string;
  description: string;
}

export interface FaqItemData {
  id: string;
  question: string;
  answer: string;
}
