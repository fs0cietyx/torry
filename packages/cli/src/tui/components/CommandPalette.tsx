import { useState } from 'react';
import { Box, Text } from 'ink';
import TextInput from 'ink-text-input';
import { TorryEngine } from '@fs0cietyx/core';
import { useAppStore } from '../store.js';

interface CommandPaletteProps {
  engine: TorryEngine;
}

export function CommandPalette({ engine }: CommandPaletteProps) {
  const [query, setQuery] = useState('');
  const [error, setError] = useState<string | null>(null);
  
  const toggleCommandPalette = useAppStore(state => state.toggleCommandPalette);

  const handleSubmit = (value: string) => {
    if (!value.trim()) {
      toggleCommandPalette();
      return;
    }
    const cleaned = value.replace(/\s+/g, '');
    
    // For now, only accept magnet links
    if (cleaned.startsWith('magnet:?')) {
      try {
        engine.context.addMagnet(cleaned, "Palette");
        toggleCommandPalette();
      } catch (err: any) {
        setError(err.message || 'Failed to add magnet link');
      }
    } else {
      setError('Only magnet links are supported right now (must start with magnet:?)');
    }
  };

  return (
    <Box 
      position="absolute" 
      top="25%"
      left="15%"
      width="70%"
      flexDirection="column"
      borderStyle="single" 
      borderColor="#8abeb7"
      paddingX={2}
      paddingY={1}
      backgroundColor="black"
    >
      <Box marginBottom={1}>
        <Text bold color="#8abeb7">COMMAND PALETTE</Text>
      </Box>
      <Box>
        <Text color="cyan">❯ </Text>
        <TextInput 
          value={query} 
          onChange={setQuery} 
          onSubmit={handleSubmit}
          placeholder="Paste magnet URI..."
        />
      </Box>
      
      {error && (
        <Box marginTop={1}>
          <Text color="red">❌ {error}</Text>
        </Box>
      )}
      
      <Box marginTop={1}>
        <Text dimColor>Press <Text bold>Enter</Text> to execute, or <Text bold>Ctrl+K</Text> to close</Text>
      </Box>
    </Box>
  );
}
