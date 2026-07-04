import { Box, Text } from 'ink';
import { useAppStore } from '../store.js';

export function HelpModal() {
  const toggleHelp = useAppStore(state => state.toggleHelp);

  return (
    <Box 
      position="absolute" 
      top={2}
      left={10}
      width="80%"
      flexDirection="column"
      borderStyle="single" 
      borderColor="#8abeb7"
      paddingX={2}
      paddingY={1}
      backgroundColor="black"
    >
      <Box marginBottom={1}>
        <Text bold color="#8abeb7">HELP / KEYBINDINGS</Text>
      </Box>

      <Box flexDirection="column" gap={1}>
        <Box>
          <Box width={15}><Text color="cyan" bold>?</Text></Box>
          <Box flexGrow={1}><Text>Toggle this help menu</Text></Box>
        </Box>
        <Box>
          <Box width={15}><Text color="cyan" bold>Tab</Text></Box>
          <Box flexGrow={1}><Text>Switch focus between Sidebar and Main pane</Text></Box>
        </Box>
        <Box>
          <Box width={15}><Text color="cyan" bold>Up / Down</Text></Box>
          <Box flexGrow={1}><Text>Navigate lists and search results</Text></Box>
        </Box>
        <Box>
          <Box width={15}><Text color="cyan" bold>/</Text></Box>
          <Box flexGrow={1}><Text>Focus the search bar (when in search pane)</Text></Box>
        </Box>
        <Box>
          <Box width={15}><Text color="cyan" bold>m</Text></Box>
          <Box flexGrow={1}><Text>Paste and download magnet link from clipboard</Text></Box>
        </Box>
        <Box>
          <Box width={15}><Text color="cyan" bold>d</Text></Box>
          <Box flexGrow={1}><Text>Download currently selected search result</Text></Box>
        </Box>
        <Box>
          <Box width={15}><Text color="cyan" bold>Ctrl+K</Text></Box>
          <Box flexGrow={1}><Text>Open the command palette</Text></Box>
        </Box>
        <Box>
          <Box width={15}><Text color="cyan" bold>Ctrl+C</Text></Box>
          <Box flexGrow={1}><Text>Quit application</Text></Box>
        </Box>
      </Box>

      <Box marginTop={1} justifyContent="center">
        <Text dimColor>Press <Text bold>Esc</Text> or <Text bold>?</Text> to close</Text>
      </Box>
    </Box>
  );
}
