import { useEffect } from 'react';
import { execSync } from 'child_process';
import { Box, Text } from 'ink';
import { useAppStore, SIDEBAR_ITEMS } from './store.js';
import type { AppAction } from './input.js';
import { useInputRouter } from './input.js';
import { Layout } from './components/Layout.js';
import { Dashboard } from './components/Dashboard.js';
import { CommandPalette } from './components/CommandPalette.js';
import { SearchPane } from './components/SearchPane.js';
import { HelpModal } from './components/HelpModal.js';
import { TorryEngine } from '@fs0cietyx/core';

interface AppProps {
  engine: TorryEngine;
  initialDownloadUrl?: string;
}

export function App({ engine, initialDownloadUrl }: AppProps) {
  const setFocus = useAppStore((state) => state.setFocus);
  const activeFocus = useAppStore((state) => state.activeFocus);
  const toggleCommandPalette = useAppStore((state) => state.toggleCommandPalette);
  const setProfileInfo = useAppStore((state) => state.setProfileInfo);
  const isCommandPaletteOpen = useAppStore((state) => state.isCommandPaletteOpen);
  const isHelpOpen = useAppStore((state) => state.isHelpOpen);
  const toggleHelp = useAppStore((state) => state.toggleHelp);
  const sidebarSelectionIndex = useAppStore((state) => state.sidebarSelectionIndex);
  const snapshot = useAppStore((state) => state.engineSnapshot);

  useEffect(() => {
    // Hydrate the UI with the runtime isolation boundaries
    setProfileInfo(engine.context.name, engine.context.dbPath);

    if (initialDownloadUrl?.startsWith('magnet:')) {
      try {
        engine.context.addMagnet(initialDownloadUrl);
      } catch (e: any) {
        console.error("Failed to add magnet:", e.message);
      }
    }

    // 10FPS FFI Snapshot Polling (Reduced from 60FPS to prevent memory leaks/CPU usage)
    const timer = setInterval(() => {
      // Clear perf hooks if Node adds them automatically in some dev environments
      if (typeof performance !== 'undefined' && performance.clearMeasures) {
        performance.clearMeasures();
      }
      
      const snap = engine.context.getSnapshot();
      useAppStore.getState().hydrateSnapshot(snap);
      
      // Auto-switch focus to Main pane and select Downloading if a torrent is added
      if (snap.torrentName && !useAppStore.getState().engineSnapshot.torrentName) {
         // This is a naive edge trigger; find index of 'lib_downloading'
         const downIdx = SIDEBAR_ITEMS.findIndex(i => i.id === 'lib_downloading');
         if (downIdx !== -1) {
           useAppStore.setState({ sidebarSelectionIndex: downIdx, activeFocus: 'main' });
         }
      }
    }, 100);
    
    return () => clearInterval(timer);
  }, [engine, setProfileInfo]);

  // Centralized action handler
  const handleAction = (action: AppAction) => {
    switch (action) {
      case 'TOGGLE_COMMAND_PALETTE':
        toggleCommandPalette();
        break;
      case 'TOGGLE_HELP':
        toggleHelp();
        break;
      case 'CANCEL':
        if (isHelpOpen) { toggleHelp(); break; }
        if (isCommandPaletteOpen) { toggleCommandPalette(); break; }
        if (activeFocus === 'main') setFocus('sidebar');
        break;
      case 'SELECT':
        if (activeFocus === 'sidebar') setFocus('main');
        break;
      case 'NEXT_PANE':
      case 'PREV_PANE':
        // Toggle between sidebar and main
        if (activeFocus === 'sidebar') setFocus('main');
        else if (activeFocus === 'main') setFocus('sidebar');
        break;
      case 'MOVE_UP':
        if (activeFocus === 'sidebar') useAppStore.getState().moveSelection('up');
        break;
      case 'MOVE_DOWN':
        if (activeFocus === 'sidebar') useAppStore.getState().moveSelection('down');
        break;
      case 'JUMP_TO_SEARCH':
        useAppStore.setState({ sidebarSelectionIndex: 0, activeFocus: 'main' });
        break;
      case 'PASTE_MAGNET':
        try {
          const clipboard = execSync('pbpaste').toString().trim();
          if (clipboard.startsWith('magnet:?')) {
            engine.context.addMagnet(clipboard, "Clipboard");
          }
        } catch (err) {}
        break;
      default:
        // Pass unhandled actions to focused components
        break;
    }
  };

  useInputRouter(handleAction);

  const selectedItem = SIDEBAR_ITEMS[sidebarSelectionIndex];

  return (
    <Layout>
      {selectedItem?.group === 'Search' ? (
        <SearchPane engine={engine} category={selectedItem.label} />
      ) : selectedItem?.id === 'lib_downloading' ? (
        <Dashboard view="downloading" engine={engine} />
      ) : selectedItem?.id === 'lib_seeding' ? (
        <Dashboard view="seeding" engine={engine} />
      ) : (
        <Box width="100%" height="100%" alignItems="center" justifyContent="center">
          <Text color="gray">Not implemented yet: {selectedItem?.label}</Text>
        </Box>
      )}

      {isCommandPaletteOpen && <CommandPalette engine={engine} />}
      {isHelpOpen && <HelpModal />}
    </Layout>
  );
}
