import { create } from 'zustand';

import type { EngineSnapshot } from '@torry/core';

export type FocusId = 'sidebar' | 'main' | 'command_palette' | 'modal';

export const SIDEBAR_ITEMS = [
  { id: 'search_all', label: 'All', group: 'Search' },
  { id: 'search_games', label: 'Games', group: 'Search' },
  { id: 'search_movies', label: 'Movies', group: 'Search' },
  { id: 'search_tv', label: 'TV', group: 'Search' },
  { id: 'search_anime', label: 'Anime', group: 'Search' },
  { id: 'lib_downloading', label: 'Downloading', group: 'Library' },
  { id: 'lib_seeding', label: 'Seeding', group: 'Library' },
];

interface AppState {
  // Engine State
  engineSnapshot: EngineSnapshot;
  hydrateSnapshot: (snapshot: EngineSnapshot) => void;

  // Focus Management
  activeFocus: FocusId;
  previousFocus: FocusId | null;
  setFocus: (id: FocusId) => void;
  restoreFocus: () => void;

  // Global UI Flags
  isCommandPaletteOpen: boolean;
  toggleCommandPalette: () => void;
  isHelpOpen: boolean;
  toggleHelp: () => void;

  // Selections
  sidebarSelectionIndex: number;
  mainSelectionIndex: number;
  moveSelection: (direction: 'up' | 'down') => void;

  // Teardown trigger
  isQuitting: boolean;
  triggerQuit: () => void;

  // Profile Context
  profileName: string;
  dbPath: string;
  setProfileInfo: (name: string, path: string) => void;
}

export const useAppStore = create<AppState>((set) => ({
  engineSnapshot: {
    torrents: []
  },
  hydrateSnapshot: (snapshot) => set({ engineSnapshot: snapshot }),

  activeFocus: 'main', // Default focus on boot
  previousFocus: null,

  setFocus: (id: FocusId) =>
    set((state) => ({
      previousFocus: state.activeFocus,
      activeFocus: id,
    })),

  restoreFocus: () =>
    set((state) => ({
      activeFocus: state.previousFocus ?? 'main',
      previousFocus: null,
    })),

  isCommandPaletteOpen: false,
  toggleCommandPalette: () =>
    set((state) => {
      const isOpening = !state.isCommandPaletteOpen;
      return {
        isCommandPaletteOpen: isOpening,
        previousFocus: isOpening ? state.activeFocus : state.previousFocus,
        activeFocus: isOpening ? 'command_palette' : (state.previousFocus ?? 'main'),
      };
    }),

  isHelpOpen: false,
  toggleHelp: () =>
    set((state) => {
      const isOpening = !state.isHelpOpen;
      return {
        isHelpOpen: isOpening,
        previousFocus: isOpening ? state.activeFocus : state.previousFocus,
        activeFocus: isOpening ? 'modal' : (state.previousFocus ?? 'main'),
      };
    }),

  sidebarSelectionIndex: 0,
  mainSelectionIndex: 0,
  moveSelection: (direction) =>
    set((state) => {
      if (state.activeFocus === 'sidebar') {
        const max = SIDEBAR_ITEMS.length - 1;
        const next = direction === 'up' ? state.sidebarSelectionIndex - 1 : state.sidebarSelectionIndex + 1;
        return { sidebarSelectionIndex: Math.max(0, Math.min(next, max)) };
      }
      if (state.activeFocus === 'main') {
        // Just a dummy max for now, can be updated by specific panes
        const max = 100;
        const next = direction === 'up' ? state.mainSelectionIndex - 1 : state.mainSelectionIndex + 1;
        return { mainSelectionIndex: Math.max(0, Math.min(next, max)) };
      }
      return {};
    }),

  isQuitting: false,
  triggerQuit: () => set({ isQuitting: true }),

  profileName: 'loading...',
  dbPath: '...',
  setProfileInfo: (profileName, dbPath) => set({ profileName, dbPath }),
}));
