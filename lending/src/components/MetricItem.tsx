import { useState, useEffect } from "react";
import type { MetricItemData } from "../utils/types";

interface MetricItemProps {
  metric: MetricItemData;
}

export function MetricItem({ metric }: MetricItemProps) {
  const [count, setCount] = useState(0);
  const [isLive, setIsLive] = useState(false);

  useEffect(() => {
    let start = 0;
    const end = metric.value;
    const isFloat = metric.id === "time";
    const step = isFloat ? 0.1 : Math.max(Math.floor(end / 60), 1);
    const intervalTime = 20;

    const timer = setInterval(() => {
      start += step;
      if (start >= end) {
        setCount(end);
        setIsLive(true);
        clearInterval(timer);
      } else {
        setCount(isFloat ? parseFloat(start.toFixed(1)) : Math.floor(start));
      }
    }, intervalTime);

    return () => clearInterval(timer);
  }, [metric.value, metric.id]);

  useEffect(() => {
    if (!isLive) return;
    const liveTimer = setInterval(() => {
      setCount((prev) => {
        if (metric.id === "tvl") {
          return prev + Math.floor(Math.random() * 21) - 10;
        }
        if (metric.id === "time") {
          const nextVal = prev + (Math.random() * 0.2 - 0.1);
          return parseFloat(Math.max(1.0, Math.min(3.0, nextVal)).toFixed(1));
        }
        return prev + (Math.random() > 0.5 ? 1 : -1);
      });
    }, 2000);
    return () => clearInterval(liveTimer);
  }, [isLive, metric.id]);

  const formatNumber = (num: number) => {
    if (metric.id === "time") return num.toFixed(1);
    return Math.floor(num).toLocaleString();
  };

  return (
    <div className="swiss-border-all p-8 flex flex-col justify-between bg-brand-bg h-[180px] select-none">
      <span className="font-mono text-xs text-brand-black/50">
        [ {metric.label} ]
      </span>
      <div className="font-mono text-4xl md:text-5xl font-bold tracking-tight text-brand-black mt-4">
        {formatNumber(count)}
        <span className="text-brand-orange text-2xl ml-1">{metric.suffix}</span>
      </div>
    </div>
  );
}
