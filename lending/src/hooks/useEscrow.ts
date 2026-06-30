import { useState } from "react";

export type EscrowStatus = "idle" | "funding" | "locked" | "validating" | "completed" | "released";

export function useEscrow() {
  const [status, setStatus] = useState<EscrowStatus>("idle");

  const startSimulation = () => {
    if (status !== "idle" && status !== "released") return;
    
    setStatus("funding");
    
    setTimeout(() => {
      setStatus("locked");
      
      setTimeout(() => {
        setStatus("validating");
        
        setTimeout(() => {
          setStatus("completed");
          
          setTimeout(() => {
            setStatus("released");
          }, 2000);
        }, 2000);
      }, 1500);
    }, 1500);
  };

  const resetSimulation = () => {
    setStatus("idle");
  };

  return { status, startSimulation, resetSimulation };
}
