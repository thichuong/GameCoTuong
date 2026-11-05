import type { EngineConfig, HistoryRecord } from '../types/game.js';

export function exportCsv(history: HistoryRecord[]): void {
  let csv = 'Turn,From,To,Piece,Captured,Note\n';
  history.forEach((record, i) => {
    const turn = i % 2 === 0 ? 'Red' : 'Black';
    const from = `(${record.from_row},${record.from_col})`;
    const to = `(${record.to_row},${record.to_col})`;
    const piece = record.piece.type;
    const captured = record.captured ? record.captured.type : '';
    const note = record.note ? `"${record.note.replace(/"/g, '""')}"` : '';
    csv += `${turn},${from},${to},${piece},${captured},${note}\n`;
  });

  const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.setAttribute('download', 'xiangqi_game.csv');
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
}

export function exportConfigJson(config: EngineConfig, filename: string): void {
  const json = JSON.stringify(config, null, 2);
  const blob = new Blob([json], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.setAttribute('download', filename);
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
}

