export const EVALUATION_CRITERIA = [
  { key: "accuracy" as const, label: "Accuracy", max: 30 },
  { key: "depth" as const, label: "Depth", max: 25 },
  { key: "sources" as const, label: "Sources", max: 20 },
  { key: "actionability" as const, label: "Actionability", max: 15 },
  { key: "presentation" as const, label: "Presentation", max: 10 },
];

export const TIMELINE_LABELS = ["Created", "Assigned", "In Progress", "Submitted", "Completed"];
