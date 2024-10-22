import React from "react";
import {
    deserializeKey,
    VisualizerAllocId,
    VisualizerContext,
    VisualizerNodeKeySerialized,
} from "../types";
import { computeLeakedState } from "../memory";
import { tally } from "../utils";

type AllocListHeaderProps = {
    context?: VisualizerContext;
};

export const AllocListHeader: React.FC<AllocListHeaderProps> = ({
    context,
}) => {
    if (!context) {
        return <>Alloc list</>;
    }
    const reachableAllocIds = new Set(
        Object.keys(context.nodes)
            .map((key) => deserializeKey(key as VisualizerNodeKeySerialized))
            .map((key) => key.alloc_id)
    );

    const leakedStatesCount = tally(
        Object.keys(context.allocs)
            .map((id) => +id as VisualizerAllocId)
            .map((id) =>
                computeLeakedState(
                    context.allocs[id].memory_kind,
                    reachableAllocIds.has(id)
                )
            )
    );

    return (
        <>
            Alloc list ({leakedStatesCount.Leaked ?? 0} leaked,{" "}
            {leakedStatesCount.Reachable ?? 0} reachable,{" "}
            {leakedStatesCount.Unleakable ?? 0} unleakable)
        </>
    );
};
