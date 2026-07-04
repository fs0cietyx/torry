import { useInput } from 'ink';
import { useAppStore } from './store.js';

/**
 * Semantic actions that the TUI can perform.
 * We decouple raw keystrokes from the actions they trigger.
 */
export type AppAction =
  | 'MOVE_UP'
  | 'MOVE_DOWN'
  | 'SELECT'
  | 'CANCEL'
  | 'TOGGLE_COMMAND_PALETTE'
  | 'TOGGLE_HELP'
  | 'NEXT_PANE'
  | 'PREV_PANE'
  | 'JUMP_TO_SEARCH'
  | 'PASTE_MAGNET'
  | 'QUIT';

/**
 * A central hook that routes raw keyboard events to semantic actions
 * based on the currently focused pane.
 *
 * This prevents `useInput()` spaghetti scattered across components.
 */
export function useInputRouter(
  actionHandler: (action: AppAction) => void
) {
  const activeFocus = useAppStore((state) => state.activeFocus);
  const triggerQuit = useAppStore((state) => state.triggerQuit);

  useInput((input, key) => {
    // 1. Global Intercepts (Highest Priority)
    if (key.ctrl && input === 'c') {
      triggerQuit();
      return;
    }

    if (key.ctrl && input === 'k') {
      actionHandler('TOGGLE_COMMAND_PALETTE');
      return;
    }

    if (input === 'q' && activeFocus !== 'main' && activeFocus !== 'command_palette') {
      triggerQuit();
      return;
    }

    if (key.tab) {
      actionHandler(key.shift ? 'PREV_PANE' : 'NEXT_PANE');
      return;
    }

    if (input === '?') {
      actionHandler('TOGGLE_HELP');
      return;
    }

    if (input === '/') {
      actionHandler('JUMP_TO_SEARCH');
      return;
    }

    if (input === 'm' && activeFocus !== 'main') {
      actionHandler('PASTE_MAGNET');
      return;
    }

    // 2. Contextual Intercepts (Based on Focus)
    switch (activeFocus) {
      case 'main':
        if (key.escape || key.leftArrow) actionHandler('CANCEL');
        if (key.upArrow || input === 'k') actionHandler('MOVE_UP');
        if (key.downArrow || input === 'j') actionHandler('MOVE_DOWN');
        if (key.return) actionHandler('SELECT');
        break;

      case 'sidebar':
        if (key.upArrow || input === 'k') actionHandler('MOVE_UP');
        if (key.downArrow || input === 'j') actionHandler('MOVE_DOWN');
        if (key.rightArrow || key.return) actionHandler('SELECT');
        break;

      case 'command_palette':
      case 'modal':
        if (key.escape) actionHandler('CANCEL');
        if (key.upArrow) actionHandler('MOVE_UP');
        if (key.downArrow) actionHandler('MOVE_DOWN');
        if (key.return) actionHandler('SELECT');
        break;
    }
  });
}
