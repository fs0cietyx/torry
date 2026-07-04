import React from 'react';
import { Box, Text } from 'ink';
import { useAppStore, SIDEBAR_ITEMS } from '../store.js';

const LOGO_LINES = [
  "▄                                                          ▄▄   ",
  "████▄▄         ▄▄▄▄▄▄▄▄▄    ▄▄▄▄▄▄▄▄▄    ▄▄▄▄▄▄▄▄▄  ▄▄▄▄▄▄ █▓█▄ ",
  "██▓░▓█        █▓░▓█ █▒▒█▌  █▓░▓█ █▒▒█▌  █▓░▓█ █▒▒█▌ ██▓░▓█ █▒▒█▌",
  "██░ ▓█       ▐█░ ▓█ █░ ░█ ▐█░ ▓█ █░ ░█ ▐█░ ▓█ █░ ░█ ██░ ▓█ ▓░▒▓█",
  "█▓  ░█▄███▄  █▓  ░█ █ ░░█ █▓  ░█ █ ░░█ █▓  ░█ █ ░░█ █▓  ░█ ▓▒░▒▓",
  "█▒ ░ █ █▓░▓▓ █▒ ░ █ █ ░░▓ █▒ ░ █  ▀▀▀  █▒ ░ █  ▀▀▀  █░▒░░█ █▓░▓▓",
  "█░▒░░█  ▀▀▀  █░▒░░█ █░ ░█ █░▒░░█       █░▒░░█       █ ░ ░█ █░ ░█",
  "█ ░ ░█       █ ░ ░█ █ ░░█  ░ ░█       █ ░ ░█       █▒░  █ █ ░░█",
  "█▒░  █       █▒░  █ █ ░░▓ █▒░  █       █▒░  █       ▐█▓░ ▓ █ ░░▓",
  "██▓░ ▓ █▀▀▀█ ▐█▓░ ▓ █   █ ▐█▓░ ▓       ▐█▓░ ▓         ▀▀▀▀▀█   █",
  " ▀▀▀▀▀▀█▄▄▄█   ▀▀▀▀▀▀▀▀▀    ▀▀▀▀         ▀▀▀▀       ▄██▀▄▄▄█▄▄▄▀"
];

const LOGO_COLORS = [
  "#cde8e5", "#bcddd9", "#abd2cd", "#9bc7c2", "#8abeb7", 
  "#79aba3", "#699990", "#8abeb7", "#4d7570", "#3c5d58", "#2b4642"
];

export function Layout({ children }: { children?: React.ReactNode }) {
  const activeFocus = useAppStore((state) => state.activeFocus);
  const sidebarSelectionIndex = useAppStore((state) => state.sidebarSelectionIndex);
  const snapshot = useAppStore((state) => state.engineSnapshot);
  const activeCount = (snapshot.torrents || []).filter((t: any) => t.progress < 100 && t.stateString !== 'COMPLETED').length;

  // Group items
  const groups: Record<string, typeof SIDEBAR_ITEMS> = {};
  SIDEBAR_ITEMS.forEach(item => {
    if (!groups[item.group]) groups[item.group] = [];
    groups[item.group]!.push(item);
  });

  return (
    <Box flexDirection="column" width="100%" height="100%" paddingX={2} paddingY={1}>
      
      {/* Top Logo Area */}
      <Box flexDirection="column" marginBottom={2} alignItems="center" width="100%">
        {LOGO_LINES.map((line, idx) => (
          <Text key={idx} color={LOGO_COLORS[idx]} bold>{line}</Text>
        ))}
      </Box>

      {/* Top Main Area */}
      <Box flexDirection="row" flexGrow={1} width="100%">
        
        {/* Sidebar */}
        <Box
          width={28}
          flexDirection="column"
          paddingRight={2}
          borderStyle={activeFocus === 'sidebar' ? 'round' : undefined}
          borderColor={activeFocus === 'sidebar' ? '#8abeb7' : undefined}
        >
          <Box flexDirection="column" gap={0}>
            {Object.entries(groups).map(([groupName, items]) => (
              <Box flexDirection="column" key={groupName} marginBottom={1}>
                <Text color="gray" bold>  {groupName}</Text>
                {items.map(item => {
                  const globalIndex = SIDEBAR_ITEMS.findIndex(i => i.id === item.id);
                  const isSelected = globalIndex === sidebarSelectionIndex;
                  return (
                    <Box key={item.id} marginLeft={2}>
                      <Text color={isSelected ? "#8abeb7" : "gray"}>
                        {isSelected ? "❯ " : "  "}
                        <Text color={isSelected ? "#fff" : "gray"}>{item.label}</Text>
                        {item.id === 'lib_downloading' && activeCount > 0 ? (
                          <Text color="#8abeb7"> ({activeCount})</Text>
                        ) : null}
                      </Text>
                    </Box>
                  );
                })}
              </Box>
            ))}
          </Box>
        </Box>

        {/* Main Content Area */}
        <Box
          flexGrow={1}
          flexDirection="column"
          borderStyle={activeFocus === 'main' ? 'round' : undefined}
          borderColor={activeFocus === 'main' ? '#8abeb7' : undefined}
          paddingX={activeFocus === 'main' ? 1 : 0}
        >
          {children}
        </Box>
      </Box>

      {/* Footer Area */}
      <Box height={1} width="100%" marginTop={1} justifyContent="flex-start" gap={3}>
        <Text>
          <Text color="#8abeb7" bold>tab </Text>
          <Text color="gray">Switch pane</Text>
        </Text>
        <Text>
          <Text color="#8abeb7" bold>↑/↓/←/→ </Text>
          <Text color="gray">Navigate</Text>
        </Text>
        <Text>
          <Text color="#8abeb7" bold>? </Text>
          <Text color="gray">Keys</Text>
        </Text>
      </Box>

    </Box>
  );
}
