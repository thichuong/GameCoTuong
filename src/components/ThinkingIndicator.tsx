import React from 'react';

interface ThinkingIndicatorProps {
  isThinking: boolean;
}

export const ThinkingIndicator: React.FC<ThinkingIndicatorProps> = ({ isThinking }) => {
  if (!isThinking) return null;

  return (
    <div className="thinking-indicator">
      <div className="spinner" />
      <span>AI đang suy nghĩ...</span>
    </div>
  );
};

