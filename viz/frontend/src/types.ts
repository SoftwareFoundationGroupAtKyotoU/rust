export type VisualizerNodeValue = {
    alloc_bytes: number[];
    messages: { severity: string; message: string }[];
};

export type VisualizerFrame = { nodes: VisualizerNodeKey[] };

export type VisualizerData = {
    nodes: [VisualizerNodeKey, VisualizerNodeValue][];
    edges: [VisualizerNodeKey, VisualizerNodeKey][];
    frames: VisualizerFrame[];
};

export type VisualizerNodeKeySerialized = string & { __type: "KEY" };

/** VisualizerData but processed */
export type VisualizerContext = {
    nodes: Record<VisualizerNodeKeySerialized, VisualizerNodeValue>;
    edges: Record<VisualizerNodeKeySerialized, VisualizerNodeKey[]>;
    frames: VisualizerFrame[];
};

export type VisualizerNodeKey = {
    alloc_id: number;
    offset: number;
    ty: string;
};

export const serializeKey = (
    key: VisualizerNodeKey
): VisualizerNodeKeySerialized =>
    JSON.stringify([
        key.alloc_id,
        key.offset,
        key.ty,
    ]) as VisualizerNodeKeySerialized;
