import { truncateAddress } from "@/shared/utils/format";
import styles from "../TaskDetail.module.css";

interface TransactionsListProps {
  hashes: Record<string, string | undefined>;
}

const TX_LABELS: Record<string, string> = {
  create: "Create TX",
  assign: "Assign TX",
  submit: "Submit TX",
  complete: "Complete TX",
};

export function TransactionsList({ hashes }: TransactionsListProps) {
  const entries = Object.entries(hashes).filter(([, v]) => !!v);
  if (entries.length === 0) return null;

  return (
    <div className={styles.section}>
      <h3 className={styles.sectionTitle}>Transactions</h3>
      <div className={styles.txList}>
        {entries.map(([key, hash]) => (
          <div key={key} className={styles.txItem}>
            <span className={styles.txLabel}>{TX_LABELS[key] ?? key}:</span>
            <a className={styles.txHash} href={`https://testnet.cspr.live/deploy/${hash}`} target="_blank" rel="noopener noreferrer">
              {truncateAddress(hash!, 10, 6)}
            </a>
          </div>
        ))}
      </div>
    </div>
  );
}
