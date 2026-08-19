import type { SnapshotDetail } from '@/shared/model/types';

export type SnapshotDetailRow = {
  label: string;
  value?: string;
  loading?: boolean;
};

export function formatSnapshotAmount(amount: string): string {
  const [whole, fraction] = amount.split('.');
  const parsed = Number(whole);
  const formatted = Number.isFinite(parsed) ? parsed.toLocaleString(undefined, { maximumFractionDigits: 0 }) : whole;
  return fraction ? `${formatted}.${fraction.slice(0, 8)}` : formatted;
}

export function snapshotTypeName(type: string): string {
  return ({ deposit: 'Deposit', withdrawal: 'Withdrawal', fee: 'Fee', rebate: 'Rebate', raw: 'Raw', transfer: 'Transfer' } as Record<string, string>)[type] || 'N/A';
}

export function snapshotDetailRows(snapshot: SnapshotDetail): SnapshotDetailRow[] {
  const positive = Number(snapshot.amount) > 0;
  const pending = `Pending confirmation (${snapshot.confirmations ?? 0}/${snapshot.assetConfirmations})`;
  const value = (label: string, candidate?: string | null): SnapshotDetailRow => ({ label, value: candidate || undefined });
  const rows = snapshot.isSafe
    ? safeSnapshotRows(snapshot, positive, pending, value)
    : accountSnapshotRows(snapshot, positive, pending, value);
  return rows.filter((row) => row.loading || row.value);
}

function accountSnapshotRows(snapshot: SnapshotDetail, positive: boolean, pending: string, value: (label: string, candidate?: string | null) => SnapshotDetailRow): SnapshotDetailRow[] {
  const kind = snapshot.snapshotType;
  return [
    value('Transaction ID', snapshot.snapshotId),
    value('Snapshot Hash', snapshot.snapshotHash),
    value('Asset Type', snapshot.assetName),
    value('Transaction Type', snapshotTypeName(kind)),
    ...(kind === 'deposit'
      ? [value('From', snapshot.sender), value('Transaction Hash', snapshot.transactionHash)]
      : kind === 'pending'
        ? [value('Status', pending), value('From', snapshot.sender), value('Transaction Hash', snapshot.transactionHash)]
        : kind === 'transfer'
          ? [value('From', positive ? snapshot.opponentName : snapshot.currentUserName), value('Receiver', positive ? snapshot.currentUserName : snapshot.opponentName)]
          : [value('Transaction Hash', snapshot.transactionHash), value(snapshot.assetTag ? 'Address' : 'Receiver', snapshot.receiver)]),
    value('Memo', snapshot.memo),
    value('Opening Balance', snapshot.openingBalance && snapshot.symbol ? `${snapshot.openingBalance} ${snapshot.symbol}` : undefined),
    value('Closing Balance', snapshot.closingBalance && snapshot.symbol ? `${snapshot.closingBalance} ${snapshot.symbol}` : undefined),
    value('Time', formatSnapshotTime(snapshot.createdAtMillis)),
    ...(kind === 'transfer' ? [value('Trace', snapshot.traceId)] : []),
  ];
}

function safeSnapshotRows(snapshot: SnapshotDetail, positive: boolean, pending: string, value: (label: string, candidate?: string | null) => SnapshotDetailRow): SnapshotDetailRow[] {
  const kind = snapshot.snapshotType;
  const rows = kind === 'pending'
    ? [value('Status', pending), value('Deposit Hash', snapshot.depositHash)]
    : [value('Transaction ID', snapshot.snapshotId), value('Transaction Hash', snapshot.transactionHash)];
  if (kind === 'transfer') {
    rows.push(value(positive ? 'From' : 'To', snapshot.opponentName || 'N/A'));
    // Flutter renders this tile twice for Safe transfers; preserve that widget flow.
    if (snapshot.memo) rows.push(value('Memo', snapshot.memo), value('Memo', snapshot.memo));
  } else if (kind === 'deposit') {
    rows.push(value('Deposit Hash', snapshot.depositHash));
  } else if (kind === 'withdrawal') {
    rows.push(value('To', snapshot.withdrawalReceiver));
    rows.push(snapshot.withdrawalHash ? value('Withdrawal Hash', snapshot.withdrawalHash) : { label: 'Withdrawal Hash', loading: true });
  }
  rows.push(value('Time', formatSnapshotTime(snapshot.createdAtMillis)));
  return rows;
}

export function formatSnapshotTime(createdAtMillis: number): string | undefined {
  if (!Number.isFinite(createdAtMillis) || createdAtMillis <= 0) return undefined;
  return new Intl.DateTimeFormat(undefined, { dateStyle: 'long', timeStyle: 'medium' }).format(new Date(createdAtMillis));
}

export function snapshotValueLines(snapshot: SnapshotDetail, fiatCurrency: string): string[] {
  const amount = Math.abs(Number(snapshot.amount));
  const price = Number(snapshot.priceUsd);
  const fiatRate = Number(snapshot.fiatRate);
  if (!Number.isFinite(amount) || amount === 0 || !Number.isFinite(price) || !Number.isFinite(fiatRate)) return [];
  const currency = /^[A-Z]{3}$/i.test(fiatCurrency) ? fiatCurrency.toUpperCase() : 'USD';
  const money = (value: number) => new Intl.NumberFormat(undefined, { style: 'currency', currency }).format(value);
  const current = amount * price * fiatRate;
  const lines = [`Value now ${money(current)} (${money(current / amount)}/${snapshot.symbol})`];
  if (snapshot.tickerPriceUsd == null || snapshot.tickerPriceUsd === '') return lines;
  const ticker = Number(snapshot.tickerPriceUsd);
  if (!Number.isFinite(ticker) || ticker === 0) return [...lines, 'Value then N/A'];
  const past = amount * ticker * fiatRate;
  return [...lines, `Value then ${money(past)} (${money(past / amount)}/${snapshot.symbol})`];
}
