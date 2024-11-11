import React from "react";
import { Foldable } from "../components/Foldable";
import { VisualizerAllocId, VisualizerContext } from "../types";
import { AllocListHeader } from "./AllocListHeader";
import { AllocEntryDisplay } from "./AllocEntryDisplay";
import { ProvenanceAllocGraph } from "./ProvenanceAllocGraph";

type VisualizerProps = { context: VisualizerContext };

export const Visualizer: React.FC<VisualizerProps> = ({ context }) => {
    return (
        <>
            <Foldable
                header={<AllocListHeader context={context} />}
                defaultFolded
            >
                <div>
                    {Object.entries(context.allocs).map(([allocId, alloc]) => (
                        <AllocEntryDisplay
                            allocId={+allocId as VisualizerAllocId}
                            alloc={alloc}
                            context={context}
                        />
                    ))}
                </div>
            </Foldable>
            <Foldable header={<>Provenance graph</>} defaultFolded>
                <div>
                    {context.provenanceFrames.map((frame) => (
                        <ProvenanceAllocGraph frame={frame} context={context} />
                    ))}
                </div>
            </Foldable>
        </>
    );
};
