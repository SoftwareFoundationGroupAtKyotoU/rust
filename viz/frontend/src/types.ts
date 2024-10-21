type NominalType<TName extends string, TValue> = TValue & { __type: TName };

export type VisualizerNodeKeySerialized = NominalType<
    "VisualizerNodeKeySerialized",
    string
>;

export type VisualizerAllocId = NominalType<"VisualizerAllocId", number>;

export type VisualizerNodeValue = {
    alloc_bytes: number[];
    messages: { severity: string; message: string }[];
};

export type VisualizerFrame = {
    description: string;
    nodes: VisualizerNodeKey[];
};

export type VisualizerAlloc = {
    bytes: number[];
};

export type VisualizerData = {
    nodes: [VisualizerNodeKey, VisualizerNodeValue][];
    edges: [VisualizerNodeKey, VisualizerNodeKey][];
    frames: VisualizerFrame[];
    allocs: Record<VisualizerAllocId, VisualizerAlloc>;
};

/** VisualizerData but processed */
export type VisualizerContext = {
    nodes: Record<VisualizerNodeKeySerialized, VisualizerNodeValue>;
    edges: Record<VisualizerNodeKeySerialized, VisualizerNodeKey[]>;
    frames: VisualizerFrame[];
    allocs: Record<VisualizerAllocId, VisualizerAlloc>;
    reachableAllocIds: Set<VisualizerAllocId>;
};

export type VisualizerNodeKey = {
    alloc_id: VisualizerAllocId;
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

// const mapReducer = <K extends string | number | symbol, V>(
//     map: Record<K, V>,
//     [k, v]: [K, V]
// ): Record<K, V> => ((map[k] = v), map);

const mapListReducer = <K extends string | number | symbol, V>(
    map: Record<K, V[]>,
    [k, v]: [K, V]
): Record<K, V[]> => ((map[k] ||= []), map[k].push(v), map);

export const toVisualizerContext = (
    data: VisualizerData
): VisualizerContext => {
    return {
        nodes: Object.fromEntries(
            data.nodes.map(([key, value]) => [serializeKey(key), value])
        ) as Record<VisualizerNodeKeySerialized, VisualizerNodeValue>,
        edges: data.edges
            .map(
                ([key, value]) =>
                    [serializeKey(key), value] as [
                        VisualizerNodeKeySerialized,
                        VisualizerNodeKey
                    ]
            )
            .reduce(mapListReducer, {}) as Record<
            VisualizerNodeKeySerialized,
            VisualizerNodeKey[]
        >,
        allocs: data.allocs,
        frames: data.frames,
        reachableAllocIds: new Set(data.nodes.map(([key]) => key.alloc_id)),
    };
};
