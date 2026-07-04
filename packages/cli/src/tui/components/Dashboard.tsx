import React, { useState, useEffect } from 'react';
import { Box, Text, useInput } from 'ink';
import { useAppStore } from '../store.js';

import type { EngineSnapshot, TorrentSnapshot } from '@fs0cietyx/core';

function formatBytes(bytes: number, decimals = 2) {
  if (!+bytes) return '0 B';
  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`;
}

function getEta(progress: number, downloaded: number, speed: number, totalBytes: number) {
  if (speed <= 0 || progress <= 0) return '';
  const remainingBytes = Math.max(0, (totalBytes > 0 ? totalBytes : 0) * (1 - (progress / 100)));
  if (remainingBytes <= 0) return '';
  const seconds = remainingBytes / speed;
  if (seconds < 60) return `${Math.floor(seconds)}s`;
  if (seconds < 3600) return `${Math.floor(seconds/60)}m ${Math.floor(seconds%60)}s`;
  return `${Math.floor(seconds/3600)}hr ${Math.floor((seconds%3600)/60)}m`;
}

function getEstimatedSize(progress: number, downloaded: number, totalBytes: number) {
  if (totalBytes > 0) return formatBytes(totalBytes);
  if (progress <= 0) return 'Unknown Size';
  return formatBytes(downloaded / (progress / 100));
}

import type { TorryEngine } from '@fs0cietyx/core';

export function Dashboard({ view = 'downloading', engine }: { view?: 'downloading' | 'recent' | 'seeding', engine?: TorryEngine }) {
  const engineSnapshot = useAppStore((state) => state.engineSnapshot);
  const activeFocus = useAppStore((state) => state.activeFocus);
  const torrents = engineSnapshot.torrents || [];
  const [localSpeedLimit, setLocalSpeedLimit] = useState<number | null>(null);
  const [shimmerPos, setShimmerPos] = useState(0);

  useEffect(() => {
    const timer = setInterval(() => {
      setShimmerPos((s) => (s + 1) % 40); // 40 steps for a 20-char bar gives an invisible pause
    }, 70);
    return () => clearInterval(timer);
  }, []);

  let active = torrents.filter((t) => t.progress < 100 && t.stateString !== 'COMPLETED');
  let completed = torrents.filter((t) => t.progress >= 100 || t.stateString === 'COMPLETED');

  if (view === 'recent') {
    active = [];
  } else if (view === 'seeding') {
    active = [];
    completed = completed.filter(t => t.stateString === 'SEEDING');
  }

  const [selectedIndex, setSelectedIndex] = useState(0);

  const displayList = view === 'downloading' ? active : completed;

  useInput((input, key) => {
    if (activeFocus !== 'main') return;
    
    if (key.upArrow) {
      setSelectedIndex((s) => Math.max(0, s - 1));
    } else if (key.downArrow) {
      setSelectedIndex((s) => Math.min(Math.max(0, displayList.length - 1), s + 1));
    }
    
    if (input === 'o' && engine) {
      engine.context.openDownloadsFolder();
    } else if (displayList.length > 0 && engine) {
      const selectedHash = displayList[selectedIndex]?.infoHash;
      if (!selectedHash) return;

      if (input === 'x') {
        try {
          (engine.context as any).cancelTorrent(selectedHash);
        } catch (e: any) {}
      } else if (input === 'p') {
        try {
          (engine.context as any).pauseTorrent(selectedHash);
        } catch (e: any) {}
      } else if (input === 'r') {
        try {
          (engine.context as any).resumeTorrent(selectedHash);
        } catch (e: any) {}
      } else if ((input === 'l' || input === 'L') && view === 'downloading') {
        const currentLimit = (engine.context as any).getDownloadSpeedLimit?.() ?? 0;
        const presets = [0, 1024*1024, 5*1024*1024, 10*1024*1024, 50*1024*1024];
        const currentIndex = presets.indexOf(currentLimit);
        const nextIndex = (currentIndex + 1) % presets.length;
        (engine.context as any).setDownloadSpeedLimit(presets[nextIndex]);
        setLocalSpeedLimit(presets[nextIndex]);
      }
    }
  });

  return (
    <Box flexDirection="column" width="100%" paddingRight={1}>
      
      {/* Active Downloads - only in downloading view */}
      {view === 'downloading' && (
      <Box flexDirection="column" width="100%" paddingX={1}>
        {active.length === 0 ? (
          <Text color="gray">No active downloads.</Text>
        ) : (
          active.map((snapshot, idx) => {
            const isSelected = idx === selectedIndex;
            const eta = getEta(snapshot.progress, snapshot.totalDownloaded, snapshot.downloadSpeed, snapshot.totalBytes);
            const sizeDisplay = getEstimatedSize(snapshot.progress, snapshot.totalDownloaded, snapshot.totalBytes);
            const barLength = 20;
            const filled = Math.floor((snapshot.progress / 100) * barLength);
            const bar = '█'.repeat(filled) + '░'.repeat(barLength - filled);
            const speedDisplay = snapshot.downloadSpeed > 0 ? `${formatBytes(snapshot.downloadSpeed)}/s` : '0 B/s';

            return (
              <Box key={snapshot.infoHash} flexDirection="column" marginY={1}>
                <Box flexDirection="row">
                  <Box width={3}>
                    <Text color="#8abeb7" bold>{isSelected ? '❯' : ' '}</Text>
                  </Box>
                  <Box width={47}><Text color={isSelected ? "#8abeb7" : "white"} bold={isSelected} wrap="truncate">{snapshot.torrentName || 'Resolving...'}</Text></Box>
                  <Box width={15}><Text color="gray">{sizeDisplay}</Text></Box>
                  <Box width={15}><Text color="#b3dfdd">{snapshot.source || 'Unknown'}</Text></Box>
                </Box>
                
                {snapshot.stateString.includes('ERROR') ? (
                  <Box flexDirection="row" marginTop={1} paddingLeft={3}>
                    <Text color="red" bold>⚠️ {snapshot.stateString.replace(/_/g, ' ')}</Text>
                  </Box>
                ) : snapshot.stateString === 'PAUSED' ? (
                  <Box flexDirection="row" marginTop={1} paddingLeft={3}>
                    <Text color="gray" bold>⏸️ PAUSED - {snapshot.progress.toFixed(1)}%</Text>
                  </Box>
                ) : (
                  <Box flexDirection="row" marginTop={1} paddingLeft={3}>
                    <Box width={22}>
                      {Array.from({ length: barLength }).map((_, i) => {
                        const isFilled = i < filled;
                        let color = isFilled ? "#8abeb7" : "gray";
                        const char = isFilled ? '█' : '░';
                        
                        if (isFilled && snapshot.stateString === 'DOWNLOADING') {
                          const distance = Math.abs(i - shimmerPos);
                          if (distance === 0) color = "#ffffff";
                          else if (distance === 1) color = "#b3dfdd";
                        }
                        
                        return <Text key={i} color={color}>{char}</Text>;
                      })}
                    </Box>
                    <Box width={8}><Text color="#8abeb7">{snapshot.progress.toFixed(1)}%</Text></Box>
                    <Box width={15}><Text color="gray">{speedDisplay}{snapshot.downloadSpeedLimit > 0 ? ` [${formatBytes(snapshot.downloadSpeedLimit)}/s]` : ''}</Text></Box>
                    <Box width={10}><Text color="gray">{snapshot.activePeers} peers</Text></Box>
                    <Box width={15}><Text color="gray">{eta}</Text></Box>
                  </Box>
                )}

                {isSelected && (
                  <Box marginTop={1} flexDirection="column" paddingLeft={3}>
                    <Box flexDirection="row" marginBottom={1}>
                      <Text color="gray">Speed Limit: </Text>
                      {[0, 1024*1024, 5*1024*1024, 10*1024*1024, 50*1024*1024].map((limit, limitIdx) => {
                        const displayLimit = localSpeedLimit !== null ? localSpeedLimit : snapshot.downloadSpeedLimit;
                        const isActive = displayLimit === limit;
                        const label = limit === 0 ? "Unlimited" : formatBytes(limit) + '/s';
                        return (
                          <Box key={limitIdx} marginRight={1}>
                            <Text color={isActive ? "#8abeb7" : "gray"} bold={isActive}>
                              {isActive ? `[${label}]` : label}
                            </Text>
                          </Box>
                        );
                      })}
                    </Box>
                    <Text color="gray">
                      Press <Text bold color="#8abeb7">p</Text> pause | <Text bold color="#8abeb7">r</Text> resume | <Text bold color="#8abeb7">x</Text> cancel | <Text bold color="#8abeb7">l</Text> cycle limit
                    </Text>
                  </Box>
                )}
              </Box>
            );
          })
        )}
      </Box>
      )}
      
      {/* Completed Downloads (Recently Downloaded / Seeding) */}
      {(view === 'downloading' || view === 'recent' || view === 'seeding') && (
      <Box 
        flexDirection="column" 
        width="100%" 
        borderStyle="round" 
        borderColor="#6b6577" 
        paddingX={1}
        paddingY={0}
        marginTop={view === 'downloading' ? 1 : 0}
      >
        <Box width="100%" marginBottom={1} flexDirection="row" justifyContent="space-between">
          <Text color="gray" bold>
            {view === 'seeding' ? `Seeding (${completed.length})` : `Recently Downloaded (${completed.length})`}
          </Text>
          <Text color="gray">[o] Open folder</Text>
        </Box>
        
        {completed.length === 0 ? (
          <Text color="gray">
             {view === 'seeding' ? 'No seeding torrents.' : 'No completed downloads yet.'}
          </Text>
        ) : (
          completed.map((snapshot, idx) => {
            const isSelected = view !== 'downloading' && idx === selectedIndex;
            return (
              <Box key={snapshot.infoHash} flexDirection="column" width="100%" marginBottom={1}>
                <Box flexDirection="row" justifyContent="space-between" width="100%">
                  <Box flexDirection="column">
                    <Text>
                      {view !== 'downloading' && <Text color="#8abeb7" bold>{isSelected ? '❯ ' : '  '}</Text>}
                      <Text color="gray">✔ </Text>
                      <Text color={isSelected ? "#8abeb7" : "white"}>{snapshot.torrentName}</Text>
                    </Text>
                    {view === 'seeding' && (
                      <Text color="gray">
                        {view !== 'downloading' ? '    ' : ''}↑ {formatBytes(snapshot.uploadSpeed)}/s  • {snapshot.activePeers} peers  • Total Up: {formatBytes(snapshot.totalUploaded)}
                      </Text>
                    )}
                  </Box>
                  <Box flexDirection="column" alignItems="flex-end">
                    <Text color={snapshot.stateString === 'SEEDING' ? '#8abeb7' : 'gray'}>{snapshot.stateString}</Text>
                    {view === 'seeding' && <Text color="gray">[p] Pause</Text>}
                  </Box>
                </Box>
                {isSelected && (
                  <Box marginTop={1} flexDirection="row" paddingLeft={view !== 'downloading' ? 4 : 0}>
                    <Text color="gray">
                      Press <Text bold color="#8abeb7">x</Text> delete
                      {snapshot.stateString === 'SEEDING' ? <Text> | <Text bold color="#8abeb7">p</Text> pause</Text> : ''}
                      {snapshot.stateString === 'PAUSED' ? <Text> | <Text bold color="#8abeb7">r</Text> resume</Text> : ''}
                    </Text>
                  </Box>
                )}
              </Box>
            );
          })
        )}
      </Box>
      )}

    </Box>
  );
}
