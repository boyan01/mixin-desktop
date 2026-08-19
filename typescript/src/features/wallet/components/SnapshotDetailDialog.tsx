import { resolveAssetUrl } from '@/shared/api/desktopApi';
import { formatSnapshotAmount, snapshotDetailRows, snapshotValueLines } from '@/features/wallet/lib/snapshotDetail';
import type { SnapshotDetail } from '@/shared/model/types';
import { classes } from '@/shared/styles/classes';

export function SnapshotDetailDialog({ snapshot, fiatCurrency, onClose }: { snapshot: SnapshotDetail; fiatCurrency: string; onClose: () => void }) {
  const amount = Number(snapshot.amount);
  const rows = snapshotDetailRows(snapshot);
  const values = snapshotValueLines(snapshot, fiatCurrency);
  return <div className={classes("modal-backdrop")} role="presentation" onMouseDown={onClose}>
    <section className={classes("snapshot-detail-dialog")} role="dialog" aria-modal="true" aria-label="Transaction detail" onMouseDown={(event) => event.stopPropagation()}>
      <header><button type="button" aria-label="Close transaction detail" onClick={onClose}>×</button></header>
      <div className={classes("snapshot-detail-header")}>
        <span className={classes("snapshot-detail-asset")}>{snapshot.assetIconUrl && <img src={resolveAssetUrl(snapshot.assetIconUrl)} alt="" />}{snapshot.chainIconUrl && <img className={classes("snapshot-detail-chain")} src={resolveAssetUrl(snapshot.chainIconUrl)} alt="" />}</span>
        <strong className={classes(snapshot.snapshotType === 'pending' ? '' : amount > 0 ? 'is-positive' : 'is-negative')}>{formatSnapshotAmount(snapshot.amount)} <small>{snapshot.symbol}</small></strong>
      </div>
      {values.length > 0 && <div className={classes("snapshot-detail-value")}>{values.map((line) => <span key={line}>{line}</span>)}</div>}
      <div className={classes("snapshot-detail-divider")} />
      <dl>{rows.map((row, index) => <div key={`${row.label}-${index}`}><dt>{row.label}</dt><dd>{row.loading ? <span className={classes("snapshot-detail-spinner")} aria-label="Loading withdrawal hash" /> : row.value}</dd></div>)}</dl>
    </section>
  </div>;
}
