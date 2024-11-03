import { MemoryKind } from "./memory";

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
    memory_kind: MemoryKind;
    bytes: number[];
};

export type VisualizerProvenanceFrame = {
    description: string;
    nodes: VisualizerAllocId[];
};

export type VisualizerData = {
    nodes: [VisualizerNodeKey, VisualizerNodeValue][];
    edges: [VisualizerNodeKey, VisualizerNodeKey][];
    frames: VisualizerFrame[];
    allocs: Record<VisualizerAllocId, VisualizerAlloc>;
    provenance_static_roots: VisualizerAllocId[];
    provenance_wildcard: VisualizerAllocId[];
    provenance_exposed: VisualizerAllocId[];
    provenance_frames: VisualizerProvenanceFrame[];
    provenance_graph: [VisualizerAllocId, VisualizerAllocId[]][];
};

/** VisualizerData but processed */
export type VisualizerContext = {
    nodes: Record<VisualizerNodeKeySerialized, VisualizerNodeValue>;
    edges: Record<VisualizerNodeKeySerialized, VisualizerNodeKey[]>;
    frames: VisualizerFrame[];
    allocs: Record<VisualizerAllocId, VisualizerAlloc>;
    reachableAllocIdByNodes: Set<VisualizerAllocId>;
    reachableAllocIdByProvenance: Set<VisualizerAllocId>;
    provenanceStaticRoots: VisualizerAllocId[];
    provenanceWildcard: VisualizerAllocId[];
    provenanceExposed: VisualizerAllocId[];
    provenanceFrames: VisualizerProvenanceFrame[];
    provenanceGraph: Record<VisualizerAllocId, VisualizerAllocId[]>;
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

export const deserializeKey = (
    keySerialized: VisualizerNodeKeySerialized
): VisualizerNodeKey => {
    const [alloc_id, offset, ty] = JSON.parse(keySerialized);
    return { alloc_id, offset, ty };
};

// const mapReducer = <K extends string | number | symbol, V>(
//     map: Record<K, V>,
//     [k, v]: [K, V]
// ): Record<K, V> => ((map[k] = v), map);

const mapListReducer = <K extends string | number | symbol, V>(
    map: Record<K, V[]>,
    [k, v]: [K, V]
): Record<K, V[]> => ((map[k] ||= []), map[k].push(v), map);

const computeReachability = (
    graph: Record<VisualizerAllocId, VisualizerAllocId[]>,
    roots: VisualizerAllocId[]
): Set<VisualizerAllocId> => {
    const reached = new Set<VisualizerAllocId>();
    roots.forEach(function dfs(current) {
        if (reached.has(current)) return;
        reached.add(current);
        graph[current]?.map(dfs);
    });
    return reached;
};

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
        // Since the graph is obtained through DFSing from roots, all must be reachable
        reachableAllocIdByNodes: new Set(
            data.nodes.map(([key]) => key.alloc_id)
        ),
        reachableAllocIdByProvenance: computeReachability(
            Object.fromEntries(data.provenance_graph),
            [
                ...data.provenance_static_roots,
                ...data.provenance_frames.flatMap((frame) => frame.nodes),
            ]
        ),
        provenanceExposed: data.provenance_exposed,
        provenanceFrames: data.provenance_frames,
        provenanceGraph: Object.fromEntries(data.provenance_graph),
        provenanceStaticRoots: data.provenance_static_roots,
        provenanceWildcard: data.provenance_wildcard,
    };
};
