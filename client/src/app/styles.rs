pub const GAME_STYLES: &str = "
                .game-layout {
                    display: flex;
                    flex-direction: column-reverse; /* Mobile: Board top (2), Log bottom (1) */
                    align-items: center;
                    gap: 20px;
                    width: 100%;
                    padding: 20px;
                    box-sizing: border-box;
                }
                
                @media (min-width: 1100px) {
                    .game-layout {
                        flex-direction: row; /* Desktop: Log left (1), Board right (2) */
                        align-items: stretch;
                        justify-content: center;
                        gap: 0; /* Remove gap on desktop to bring log closer */
                    }
                }

                .log-panel {
                    width: 90%;
                    max-width: 500px; /* Wider on mobile */
                    height: 60vh; /* Dynamic height */
                    max-height: 600px;
                    background: #333;
                    border-radius: 8px;
                    box-shadow: 0 4px 6px rgba(0,0,0,0.3);
                    display: flex;
                    flex-direction: column;
                    border: 1px solid #444;
                }
                
                @media (min-width: 1100px) {
                    .log-panel {
                        width: 480px;
                        height: auto; /* Stretch to match board */
                        margin-top: 45px; /* Align with board canvas (skip captured pieces) */
                    }
                }

                .log-header {
                    background: #444;
                    color: #f0d9b5;
                    padding: 15px;
                    font-weight: bold;
                    text-align: center;
                    border-bottom: 1px solid #555;
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                }

                .log-list {
                    flex: 1;
                    overflow-y: auto;
                    padding: 0;
                    margin: 0;
                    list-style: none;
                    scrollbar-width: thin;
                    scrollbar-color: #666 #333;
                }
                
                .log-list::-webkit-scrollbar {
                    width: 8px;
                }
                .log-list::-webkit-scrollbar-track {
                    background: #333;
                }
                .log-list::-webkit-scrollbar-thumb {
                    background-color: #666;
                    border-radius: 4px;
                }

                .log-item {
                    padding: 10px 15px;
                    border-bottom: 1px solid #444;
                    font-size: 14px;
                    display: flex;
                    flex-direction: column;
                    gap: 4px;
                }

                .log-item:nth-child(even) {
                    background-color: #3a3a3a;
                }
                
                .log-item:last-child {
                    border-left: 3px solid #f0d9b5;
                    background-color: #444;
                }

                .move-info {
                    display: flex;
                    justify-content: space-between;
                    font-weight: 500;
                }
                
                .ai-stats {
                    font-size: 0.85em;
                    color: #aaa;
                    font-family: monospace;
                }

                /* New Controls Design */
                .controls-area {
                    display: flex;
                    flex-direction: column;
                    gap: 15px;
                    width: 90%;
                    /* Removed max-width to let it fit the screen naturally */
                    margin: 20px auto;
                    padding: 20px;
                    background: #2a2a2a;
                    border-radius: 12px;
                    box-shadow: 0 4px 6px rgba(0,0,0,0.2);
                    border: 1px solid #444;
                    box-sizing: border-box;
                }

                .controls-config {
                    display: flex;
                    gap: 15px;
                    width: 100%;
                }

                .controls-actions {
                    display: grid;
                    grid-template-columns: 1fr 1fr;
                    gap: 15px;
                    width: 100%;
                }

                .control-group {
                    display: flex;
                    flex-direction: column;
                    gap: 5px;
                    flex: 1; /* Config items take equal width */
                }

                .control-label {
                    font-size: 0.85em;
                    color: #aaa;
                    margin-left: 2px;
                }

                select, button.control-btn {
                    width: 100%;
                    padding: 10px 14px;
                    border-radius: 8px;
                    border: 1px solid #555;
                    background: #3a3a3a;
                    color: #eee;
                    font-size: 14px;
                    cursor: pointer;
                    transition: all 0.2s ease;
                    outline: none;
                    font-family: inherit;
                    box-sizing: border-box; /* Ensure padding doesn't affect width */
                }

                select:hover, button.control-btn:hover {
                    background: #4a4a4a;
                    border-color: #777;
                    transform: translateY(-1px);
                    box-shadow: 0 2px 4px rgba(0,0,0,0.2);
                }
                
                select:focus, button.control-btn:focus {
                    border-color: #a8e6cf;
                    box-shadow: 0 0 0 2px rgba(168, 230, 207, 0.2);
                }

                button.btn-primary {
                    background: #4CAF50;
                    color: white;
                    border: none;
                }
                button.btn-primary:hover {
                    background: #45a049;
                }

                button.btn-warning {
                    background: #FF9800;
                    color: black;
                    border: none;
                    font-weight: 500;
                }
                button.btn-warning:hover {
                    background: #f57c00;
                }
                
                button.btn-danger {
                    background: #f44336;
                    color: white;
                    border: none;
                }
                 button.btn-danger:hover {
                    background: #d32f2f;
                }

                button.btn-info {
                    background: #2196F3;
                    color: white;
                    border: none;
                }
                button.btn-info:hover {
                    background: #1976D2;
                }

                @media (max-width: 480px) {
                    .controls-area {
                         /* Mobile tweaks if needed, but flex/grid generally handle it */
                         padding: 15px;
                    }
                }


                .thinking-indicator {
                    display: flex;
                    align-items: center;
                    justify-content: center;
                    gap: 10px;
                    color: #a8e6cf;
                    font-weight: bold;
                    margin: 10px 0;
                    height: 24px;
                    animation: pulse 1.5s infinite;
                }

                @keyframes pulse {
                    0% { opacity: 0.6; }
                    50% { opacity: 1; }
                    100% { opacity: 0.6; }
                }

                .config-panel {
                    display: flex;
                    gap: 20px;
                    width: 100%;
                    max-width: 1000px;
                    margin-top: 20px;
                    margin-bottom: 20px;
                    flex-wrap: wrap;
                    justify-content: center;
                }

                .config-column {
                    flex: 1;
                    min-width: 300px;
                    background: #333;
                    padding: 15px;
                    border-radius: 8px;
                    border: 1px solid #444;
                }

                .config-title {
                    color: #f0d9b5;
                    font-weight: bold;
                    text-align: center;
                    margin-bottom: 15px;
                    border-bottom: 1px solid #555;
                    padding-bottom: 10px;
                }

                .captured-panel {
                    padding: 10px;
                    background: #3a3a3a;
                    border-bottom: 1px solid #555;
                    display: flex;
                    flex-direction: column;
                    gap: 5px;
                }

                .captured-row {
                    display: flex;
                    align-items: center;
                    gap: 10px;
                    font-size: 0.9em;
                }

                .captured-label {
                    width: 60px;
                    font-weight: bold;
                    color: #aaa;
                }

                .captured-pieces {
                    display: flex;
                    flex-wrap: wrap;
                    gap: 2px;
                }

                .captured-piece {
                    width: 24px;
                    height: 24px;
                    border-radius: 50%;
                    background: #f0d9b5;
                    display: flex;
                    justify-content: center;
                    align-items: center;
                    font-family: 'KaiTi', '楷体', serif;
                    font-weight: bold;
                    font-size: 16px;
                    line-height: 1;
                    border: 1px solid #5c3a1e;
                }

                .layout-spacer {
                    display: none;
                }

                .side-column {
                    display: contents; /* Mobile: just show content */
                }

                @media (min-width: 1100px) {
                    .side-column {
                        display: flex;
                        flex: 1;
                        min-width: 0; /* Allow shrinking */
                    }
                    
                    /* Left column aligns right (next to board) */
                    .side-column.left {
                        justify-content: flex-end;
                        padding-right: 5px; /* Minimal padding */
                    }
                    
                    /* Right column aligns left (next to board) */
                    .side-column.right {
                        justify-content: flex-start;
                        padding-left: 5px; /* Minimal padding */
                    }

                    .log-panel {
                        width: 480px;
                        min-width: 350px; /* Prevent it from becoming too narrow */
                        height: auto; /* Stretch to match board */
                        margin-top: 45px; /* Align with board canvas (skip captured pieces) */
                    }
                }
";
