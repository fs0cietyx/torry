import { version, addViaRust } from '@torry/core';

const BANNER = `
  ████████╗ ██████╗ ██████╗ ██████╗ ██╗   ██╗
  ╚══██╔══╝██╔═══██╗██╔══██╗██╔══██╗╚██╗ ██╔╝
     ██║   ██║   ██║██████╔╝██████╔╝ ╚████╔╝
     ██║   ██║   ██║██╔══██╗██╔══██╗  ╚██╔╝
     ██║   ╚██████╔╝██║  ██║██║  ██║   ██║
     ╚═╝    ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝   ╚═╝
`;

console.log(BANNER);
console.log(`  Torry v${version()}`);
console.log(`  Rust bridge verified: 2 + 3 = ${addViaRust(2, 3)}`);
console.log();
