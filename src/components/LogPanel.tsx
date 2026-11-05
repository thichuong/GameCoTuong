import React, { useRef, useEffect } from 'react';
import type { HistoryRecord } from '../types/game.js';

interface LogPanelProps {
  history: HistoryRecord[];
}

export const LogPanel: React.FC<LogPanelProps> = ({ history }) => {
  const logEndRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [history.length]);

  return (
    <div className="log-panel">
      <div className="log-header">
        <span>📜 Biên bản ván đấu</span>
        <span className="log-count">{history.length} nước</span>
      </div>
      <div className="log-list-container">
        {history.length === 0 ? (
          <div className="log-empty">Chưa có nước đi nào</div>
        ) : (
          <ul className="log-list">
            {history.map((record, i) => {
              const turnNum = Math.floor(i / 2) + 1;
              const isRed = i % 2 === 0;
              const sideIcon = isRed ? '🔴' : '⚫';

              return (
                <li key={`${i}-${record.from_row}-${record.from_col}-${record.to_row}-${record.to_col}`} className="log-item">
                  <div className="move-info">
                    <span className="move-title">
                      {turnNum}. {sideIcon} ({record.from_row},{record.from_col}) → ({record.to_row},{record.to_col})
                    </span>
                    <span className="piece-type-badge">{record.piece.type}</span>
                  </div>
                  {record.captured && (
                    <div className="captured-badge">
                      Ăn quân: {record.captured.type}
                    </div>
                  )}
                  {record.note && (
                    <div className="ai-stats-badge">{record.note}</div>
                  )}
                </li>
              );
            })}
            <div ref={logEndRef} />
          </ul>
        )}
      </div>
    </div>
  );
};

