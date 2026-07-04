import { useState, useEffect } from 'react';
import { Box, Text, useInput } from 'ink';
import TextInput from 'ink-text-input';
import { TorryEngine } from '@fs0cietyx/core';
import { execSync } from 'child_process';
import { useAppStore } from '../store.js';

import { SOURCES } from '../../search/registry.js';
import type { TorrentResult, Source } from '../../search/types.js';

interface SearchPaneProps {
  engine: TorryEngine;
  category: string;
}

function formatBytes(bytes: number, decimals = 2) {
  if (!+bytes) return '0 B';
  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`;
}

export function SearchPane({ engine, category }: SearchPaneProps) {
  const [query, setQuery] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [results, setResults] = useState<TorrentResult[] | null>(null);
  const [offlineSources, setOfflineSources] = useState<string[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [isSearchFocused, setIsSearchFocused] = useState(true);
  
  const activeFocus = useAppStore(state => state.activeFocus);
  const isCommandPaletteOpen = useAppStore(state => state.isCommandPaletteOpen);
  const isActuallyFocused = isSearchFocused && activeFocus === 'main' && !isCommandPaletteOpen;

  useEffect(() => {
    setResults(null);
    setQuery('');
    setSelectedIndex(0);
    setError(null);
    setOfflineSources([]);
    setIsSearchFocused(true);
  }, [category]);

  const getSourcesToSearch = (): Source[] => {
    if (category === 'All') return [...SOURCES];
    if (category === 'YTS') return SOURCES.filter((s: Source) => s.id === 'yts');
    if (category === 'FZ') return SOURCES.filter((s: Source) => s.id === 'fitgirl');
    if (category === 'EZTV') return SOURCES.filter((s: Source) => s.id === 'eztv');
    return SOURCES.filter((s: Source) => s.group === category);
  };

  useInput((input, key) => {
    // Only handle inputs if this pane is actually active
    if (activeFocus !== 'main') return;

    // If input is focused, let TextInput handle it, except escape
    if (isSearchFocused) {
      if (key.escape) {
        if (results !== null) {
          setIsSearchFocused(false);
        } else if (query !== '') {
          setQuery(''); // Just clear the query if no results but there is text
        } else {
          setIsSearchFocused(false); // If query is already empty, blur the input
        }
      }
      return;
    }

    if (input === '/' || key.escape) {
      if (key.escape) {
        setResults(null);
      }
      setIsSearchFocused(true);
      return;
    }

    if (input === 'm') {
      try {
        const clipboard = execSync('pbpaste').toString().trim();
        if (clipboard.startsWith('magnet:?')) {
          engine.context.addMagnet(clipboard, "Clipboard");
          setError(null);
        } else {
          setError('Clipboard does not contain a valid magnet link');
        }
      } catch (err: any) {
        setError('Failed to read clipboard');
      }
      return;
    }

    if (results !== null) {
      if (key.upArrow) {
        setSelectedIndex(Math.max(0, selectedIndex - 1));
      }
      if (key.downArrow) {
        setSelectedIndex(Math.min(results.length - 1, selectedIndex + 1));
      }
      if (key.escape) {
        setResults(null);
        setIsSearchFocused(true);
      }
      if (input === 'd' && results.length > 0) {
        const selected = results[selectedIndex];
        if (selected) {
          try {
            if (selected.magnet.startsWith('magnet:?')) {
               engine.context.addMagnet(selected.magnet, selected.source);
            } else {
               const trackers = [
                 'http://tracker.opentrackr.org:1337/announce',
                 'udp://tracker.opentrackr.org:1337/announce',
                 'udp://tracker.openbittorrent.com:6969/announce'
               ];
               const trString = trackers.map(t => `&tr=${encodeURIComponent(t)}`).join('');
               const m = `magnet:?xt=urn:btih:${selected.infoHash}&dn=${encodeURIComponent(selected.name)}${trString}`;
               engine.context.addMagnet(m, selected.source);
            }
          } catch (err: any) {
            setError(err.message || 'Failed to add magnet link');
          }
        }
      }
    }
  });

  const handleSubmit = async (value: string) => {
    setError(null);
    setIsSearchFocused(false); // Blur search input, move focus to results
    
    if (value.startsWith('magnet:?')) {
      try {
        engine.context.addMagnet(value, "Direct");
      } catch (err: any) {
        setError(err.message || 'Failed to add magnet link');
      }
    } else {
      setLoading(true);
      try {
        const sources = getSourcesToSearch();
        const offline: string[] = [];
        
        const promises = sources.map((s: Source) => 
          s.search(value, { signal: AbortSignal.timeout(10000) })
           .catch(() => {
              offline.push(s.label || s.id);
              return [];
           })
        );
        
        const resultsArrays = await Promise.all(promises);
        const combined = resultsArrays.flat().sort((a: TorrentResult, b: TorrentResult) => b.seeders - a.seeders);
        
        setResults(combined.slice(0, 15));
        setSelectedIndex(0);
        setOfflineSources(offline);
      } catch (err: any) {
        setError(err.message || 'Search error');
      } finally {
        setLoading(false);
      }
    }
  };

  return (
    <Box width="100%" height="100%" flexDirection="column">
      
      {/* Top Search Bar */}
      <Box flexDirection="column" width="100%" marginBottom={1}>
        <Text color="#8abeb7" bold>Search {category}</Text>
        <Box 
          width="100%"
          borderStyle="round"
          borderColor={isActuallyFocused ? "#8abeb7" : "gray"}
        >
          {isActuallyFocused ? (
            <TextInput 
              value={query} 
              onChange={setQuery} 
              onSubmit={handleSubmit}
              placeholder={loading ? "Searching..." : `Type query or magnet link...`}
              focus={true}
            />
          ) : (
            <Text color={query ? "white" : "gray"}>
              {query || (loading ? "Searching..." : "Type query or magnet link...")}
            </Text>
          )}
        </Box>
      </Box>

      {error && (
        <Box marginBottom={1}>
          <Text color="red">❌ {error}</Text>
        </Box>
      )}

      {/* Results View */}
      {results !== null && (
        <Box flexDirection="column" width="100%" flexGrow={1} borderStyle="single" borderColor="gray" paddingX={1}>
          
          <Box flexDirection="row" justifyContent="space-between" marginBottom={1}>
            <Text color="#8abeb7" bold>Results for "{query}"</Text>
            <Text color="gray">
              <Text bold color="#8abeb7">/</Text> search | <Text bold color="#8abeb7">m</Text> magnet | <Text bold color="#8abeb7">d</Text> save
            </Text>
          </Box>

          {offlineSources.length > 0 && results.length > 0 && (
            <Box paddingY={1}>
              <Text color="cyan" wrap="truncate-end">Offline sources: {offlineSources.join(', ')}</Text>
            </Box>
          )}
          
          <Box flexDirection="column" width="100%" flexGrow={1}>
            {/* Header Row */}
            {results.length > 0 && (
              <Box paddingX={1} marginBottom={1}>
                <Box width={3}><Text> </Text></Box>
                <Box flexGrow={1} flexShrink={1} flexBasis={0}><Text color="gray" bold wrap="truncate">NAME</Text></Box>
                <Box width={12} justifyContent="flex-end"><Text color="gray" bold>SIZE</Text></Box>
                <Box width={12} justifyContent="flex-end"><Text color="gray" bold>SEED:LCH</Text></Box>
                <Box width={12} justifyContent="flex-end"><Text color="gray" bold>SOURCE</Text></Box>
              </Box>
            )}
            
            {results.length === 0 ? (
              <Box padding={1}><Text color="gray">No results found.</Text></Box>
            ) : (
              results.map((r, i) => (
                <Box key={r.infoHash + i} paddingX={1} backgroundColor={i === selectedIndex ? '#23413d' : undefined}>
                  <Box width={3}>
                    <Text color="#8abeb7">{i === selectedIndex ? '❯' : ' '}</Text>
                  </Box>
                  <Box flexGrow={1} flexShrink={1} flexBasis={0}>
                    <Text color={i === selectedIndex ? '#fff' : '#e9e4f5'} wrap="truncate">{r.name}</Text>
                  </Box>
                  <Box width={12} justifyContent="flex-end">
                    <Text color="gray">{formatBytes(r.sizeBytes)}</Text>
                  </Box>
                  <Box width={12} justifyContent="flex-end">
                    <Text color="#8abeb7">{String(r.seeders ?? 0)}</Text>
                    <Text color="gray">:</Text>
                    <Text color="#f87171">{String(r.leechers ?? 0)}</Text>
                  </Box>
                  <Box width={12} justifyContent="flex-end">
                    <Text color="#b3dfdd">{r.source}</Text>
                  </Box>
                </Box>
              ))
            )}
          </Box>
        </Box>
      )}

      {/* If no results yet but not focused and no results, hint to search */}
      {!isSearchFocused && results === null && (
        <Box width="100%" flexGrow={1} justifyContent="center" alignItems="center">
           <Text color="gray">Press <Text bold color="#8abeb7">/</Text> to search or <Text bold color="#8abeb7">m</Text> to paste a magnet link</Text>
        </Box>
      )}
    </Box>
  );
}
