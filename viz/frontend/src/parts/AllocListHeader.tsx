import React from "react";
import {
    deserializeKey,
    VisualizerContext,
    VisualizerNodeKeySerialized,
} from "../types";

type AllocListHeaderProps = {
    context?: VisualizerContext;
};

export const AllocListHeader: React.FC<AllocListHeaderProps> = ({
    context,
}) => {
    if (!context) {
        return <>Alloc list</>;
    }
    const allocIds = new Set(Object.keys(context.allocs).map((id) => +id));
    const reachableAllocIds = new Set(
        Object.keys(context.nodes)
            .map((key) => deserializeKey(key as VisualizerNodeKeySerialized))
            .map((key) => key.alloc_id)
            .filter((id) => allocIds.has(id))
    );
    console.log(allocIds, reachableAllocIds);
    return (
        <>
            Alloc list ({reachableAllocIds.size}/{allocIds.size} reachable)
        </>
    );
};
