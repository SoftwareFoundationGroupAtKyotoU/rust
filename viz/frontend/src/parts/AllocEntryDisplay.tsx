import clsx from "clsx";
import React from "react";
import { BytesDisplay } from "../components/BytesDisplay";
import { Foldable } from "../components/Foldable";
import {
    VisualizerAlloc,
    VisualizerAllocId,
    VisualizerContext,
} from "../types";
import { computeLeakedState } from "../memory";
import { TextFold } from "../components/TextFold";

type AllocEntryDisplayProps = {
    context: VisualizerContext;
    allocId: VisualizerAllocId;
    alloc: VisualizerAlloc;
};

export const AllocEntryDisplay: React.FC<AllocEntryDisplayProps> = ({
    context,
    allocId,
    alloc,
}) => {
    const reachable = context?.reachableAllocIdByProvenance.has(
        +allocId as VisualizerAllocId
    );
    const leakedState = computeLeakedState(alloc.memory_kind, reachable);

    return (
        <div
            className={clsx({
                "text-blue-600": leakedState === "Unleakable",
                "text-red-600": leakedState === "Leaked",
                "text-green-600": leakedState === "Reachable",
            })}
        >
            <Foldable
                header={
                    <>
                        Alloc {allocId} ({alloc.bytes.length} bytes,{" "}
                        {alloc.memory_kind})
                    </>
                }
            >
                {/* Check reachable or not through the alloc graph */}
                <div className="flex gap-[1ch]">
                    <span>bytes:</span>
                    <div>
                        <BytesDisplay bytes={alloc.bytes} />
                    </div>
                </div>
                <div className="flex gap-[1ch]">
                    <span>backtrace:</span>
                    <div>
                        {alloc.backtrace ? (
                            <TextFold maxLength={50} text={alloc.backtrace} />
                        ) : (
                            "(none)"
                        )}
                    </div>
                </div>
            </Foldable>
        </div>
    );
};
