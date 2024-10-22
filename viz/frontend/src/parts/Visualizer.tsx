import clsx from "clsx";
import React from "react";
import { Foldable } from "../components/Foldable";
import { VisualizerAllocId, VisualizerContext } from "../types";
import { AllocGraph } from "./AllocGraph";
import { AllocListHeader } from "./AllocListHeader";
import { BytesDisplay } from "../components/BytesDisplay";

type VisualizerProps = { context: VisualizerContext };

export const Visualizer: React.FC<VisualizerProps> = ({ context }) => {
    return (
        <>
            <Foldable
                header={<AllocListHeader context={context} />}
                defaultFolded
            >
                <div>
                    {Object.entries(context?.allocs ?? {}).map(
                        ([allocId, alloc]) => (
                            <div
                                className={clsx({
                                    "text-red-600":
                                        !context?.reachableAllocIds.has(
                                            +allocId as VisualizerAllocId
                                        ),
                                    "text-green-600":
                                        context?.reachableAllocIds.has(
                                            +allocId as VisualizerAllocId
                                        ),
                                })}
                            >
                                <Foldable
                                    header={`Alloc ${allocId} (${alloc.bytes.length} bytes)`}
                                >
                                    {/* Check reachable or not through the alloc graph */}
                                    <div className="flex gap-[1ch]">
                                        <span>bytes:</span>
                                        <div>
                                            <BytesDisplay bytes={alloc.bytes} />
                                        </div>
                                    </div>
                                </Foldable>
                            </div>
                        )
                    )}
                </div>
            </Foldable>
            <Foldable header={<>Alloc graph</>} defaultFolded>
                <div>
                    {context?.frames.map((frame) => (
                        <Foldable header={<>frame {frame.description}</>}>
                            {frame.nodes.map((node) => (
                                <AllocGraph
                                    nodeKey={node}
                                    context={context}
                                    ancestors={[]}
                                />
                            ))}
                        </Foldable>
                    ))}
                </div>
            </Foldable>
        </>
    );
};
