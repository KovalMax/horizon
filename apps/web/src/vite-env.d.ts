interface ImportMetaEnv {
  readonly PROD: boolean;
  readonly VITE_OSM_TILE_URL?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
