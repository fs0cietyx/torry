declare module '@fs0cietyx/binding' {
  export interface PeerStateSnapshot {
    ip: string;
    unchoked: boolean;
    blocksInFlight: number;
  }

  export interface TorrentSnapshot {
    infoHash: string;
    totalDownloaded: number;
    totalUploaded: number;
    downloadSpeed: number;
    uploadSpeed: number;
    activePeers: number;
    stateString: string;
    torrentName: string;
    progress: number;
    peers: PeerStateSnapshot[];
    pieceMap: number[];
    totalPieces: number;
    totalBytes: number;
    source: string;
    downloadSpeedLimit: number;
    uploadSpeedLimit: number;
  }

  export interface EngineSnapshot {
    torrents: TorrentSnapshot[];
  }

  export class RuntimeContext {
    name: string;
    dbPath: string;
    torrentsDir: string;
    downloadsDir: string;
    cacheDir: string;

    constructor(profileName: string);
    getSnapshot(): EngineSnapshot;
    addMagnet(uri: string, source?: string | undefined | null): void;
    cancelTorrent(infoHash: string): void;
    openDownloadsFolder(): void;
    setDownloadSpeedLimit(bytesPerSecond: number): void;
    getDownloadSpeedLimit(): number;
  }
}
