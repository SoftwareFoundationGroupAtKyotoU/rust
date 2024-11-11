import { MemoryKind } from "./memory";

type NominalType<TName extends string, TValue> = TValue & { __type: TName };

export type VisualizerAllocId = NominalType<"VisualizerAllocId", number>;

export type VisualizerAlloc = {
    memory_kind: MemoryKind;
    backtrace?: string;
    bytes: number[];
};

export type VisualizerProvenanceFrame = {
    description: string;
    nodes: VisualizerAllocId[];
};

export type VisualizerData = {
    allocs: Record<VisualizerAllocId, VisualizerAlloc>;
    provenance_static_roots: VisualizerAllocId[];
    provenance_wildcard: VisualizerAllocId[];
    provenance_exposed: VisualizerAllocId[];
    provenance_frames: VisualizerProvenanceFrame[];
    provenance_graph: [VisualizerAllocId, VisualizerAllocId[]][];
};

/** VisualizerData but processed */
export type VisualizerContext = {
    allocs: Record<VisualizerAllocId, VisualizerAlloc>;
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
        allocs: data.allocs,
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
