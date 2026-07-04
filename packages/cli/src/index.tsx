import { render } from 'ink';
import { Command } from 'commander';
import { version, TorryEngine } from '@torry/core';
import { App } from './tui/App.js';

// ─── 1. TTY & CI Detection ────────────────────────────────────────────────
const isInteractive = process.stdout.isTTY && !process.env.CI;

// ─── 2. Commander Initialization ─────────────────────────────────────────
const program = new Command();

program
  .name('torry')
  .description('Torry — A modern, terminal-native decentralized download manager.')
  .version(version())
  .option('-p, --profile <name>', 'Use a specific isolated profile', 'default');

// ─── 3. Explicit Subcommands (UNIX Mode) ─────────────────────────────────

program
  .command('download <url>')
  .description('Headless download mode (pipe-friendly, scriptable)')
  .option('--json', 'Output results as JSON for machine parsing')
  .option('--quiet', 'Suppress all non-error output')
  .action((url, options) => {
    // This is the UNIX boundary. No Ink. No TUI. 
    // We instantiate the headless engine, listen to events, and write to stdout.
    if (options.json) {
      // Setup JSON-only event listeners...
      console.log(JSON.stringify({ status: 'starting', url }));
    } else {
      // Setup inline progress bar (if TTY) or silent (if piped)...
      console.log(`Downloading ${url}... (Headless mode)`);
    }
  });

program
  .command('search <query>')
  .description('Headless search mode')
  .option('--json', 'Output results as JSON')
  .action((query, options) => {
    if (options.json) {
      console.log(JSON.stringify([{ title: 'Ubuntu 24.04', size: '2.4GB' }]));
    } else {
      console.log(`Searching for: ${query}`);
      console.log('1. Ubuntu 24.04 (2.4GB)');
    }
  });

// ─── 4. Smart Argument Detection & App Boot ──────────────────────────────

program
  .argument('[url]', 'Optional magnet link or URL for direct download')
  .action((url, options) => {
    launchTUI({ profile: options.profile, initialDownloadUrl: url });
  });

program.parse(process.argv);

// ─── 5. TUI App Launcher ─────────────────────────────────────────────────

function launchTUI(options: { profile: string, initialDownloadUrl?: string }) {
  if (!isInteractive) {
    console.error('❌ Error: Cannot launch TUI in a non-interactive environment.');
    process.exit(1);
  }
  
  let engine;
  try {
    // Attempt to acquire the profile lock and initialize paths
    engine = new TorryEngine(options.profile);
  } catch (error: any) {
    console.error(`\n❌ Failed to boot Torry in profile: '${options.profile}'`);
    console.error(`Reason: ${error.message}\n`);
    process.exit(1);
  }

  // Enter alternate screen buffer
  process.stdout.write('\x1b[?1049h');
  
  // Ensure we exit the alternate screen buffer on unexpected exits
  const cleanup = () => process.stdout.write('\x1b[?1049l');
  process.on('exit', cleanup);

  // Render the TUI, passing the isolated engine instance
  const { waitUntilExit } = render(<App engine={engine} />);
  
  waitUntilExit().then(() => {
    engine.shutdown().then(() => {
      cleanup();
      process.removeListener('exit', cleanup);
      process.exit(0);
    });
  });
}
